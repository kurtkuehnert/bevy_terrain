use crate::{
    terrain::TerrainComponents,
    terrain_data::{GpuTileAtlas, TileAtlas},
    util::GpuBuffer,
};
use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    pbr::{MeshTransforms, MeshUniform, PreviousGlobalTransform},
    prelude::*,
    render::{
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{binding_types::*, *},
        renderer::{RenderDevice, RenderQueue},
        texture::FallbackImage,
        Extract,
    },
};
use itertools::Itertools;
use std::iter;

pub(crate) fn create_terrain_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::all(),
            (
                storage_buffer_read_only::<MeshUniform>(false), // mesh
                uniform_buffer::<TerrainConfigUniform>(false),  // terrain
                uniform_buffer::<AttachmentUniform>(false),     // attachments
                sampler(SamplerBindingType::Filtering),         // terrain_sampler
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment_0
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment_1
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment_2
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment_3
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment_4
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment_5
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment_6
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment_7
            ),
        ),
    )
}

// Todo: replace the TerrainBindGroup with this AsBindGroup derive
// Try implementing manually for now?
#[derive(AsBindGroup)]
pub struct Terrain {}

#[derive(Default, ShaderType)]
struct AttachmentConfig {
    size: f32,
    scale: f32,
    offset: f32,
    _padding: u32,
}

#[derive(Default, ShaderType)]
struct AttachmentUniform {
    data: [AttachmentConfig; 8],
}

impl AttachmentUniform {
    fn new(tile_atlas: &GpuTileAtlas) -> Self {
        let mut uniform = Self::default();

        for (config, attachment) in iter::zip(&mut uniform.data, &tile_atlas.attachments) {
            config.size = attachment.buffer_info.center_size as f32;
            config.scale = attachment.buffer_info.center_size as f32
                / attachment.buffer_info.texture_size as f32;
            config.offset = attachment.buffer_info.border_size as f32
                / attachment.buffer_info.texture_size as f32;
        }

        uniform
    }
}

/// The terrain config data that is available in shaders.
#[derive(Default, ShaderType)]
struct TerrainConfigUniform {
    lod_count: u32,
    scale: f32,
}

impl TerrainConfigUniform {
    fn from_tile_atlas(tile_atlas: &TileAtlas) -> Self {
        Self {
            lod_count: tile_atlas.lod_count,
            scale: tile_atlas.model.scale() as f32,
        }
    }
}

// Todo: convert the mesh buffer to a ShaderStorageBuffer and merge with the terrain config buffer

pub struct GpuTerrain {
    mesh_buffer: GpuBuffer<MeshUniform>,
    pub(crate) terrain_bind_group: BindGroup,
}

impl GpuTerrain {
    fn new(
        device: &RenderDevice,
        fallback_image: &FallbackImage,
        tile_atlas: &TileAtlas,
        gpu_tile_atlas: &GpuTileAtlas,
    ) -> Self {
        let mesh_buffer = GpuBuffer::empty_sized_labeled(
            None,
            device,
            MeshUniform::SHADER_SIZE.get(),
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let terrain_config_buffer = GpuBuffer::create(
            device,
            &TerrainConfigUniform::from_tile_atlas(tile_atlas),
            BufferUsages::UNIFORM,
        );

        let atlas_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            anisotropy_clamp: 16, // Todo: make this customisable
            ..default()
        });

        let attachments = (0..8)
            .map(|i| {
                gpu_tile_atlas
                    .attachments
                    .get(i)
                    .map_or(fallback_image.d2_array.texture_view.clone(), |attachment| {
                        attachment.atlas_texture.create_view(&default())
                    })
            })
            .collect_vec();

        let attachment_uniform = AttachmentUniform::new(gpu_tile_atlas);
        let attachment_buffer =
            GpuBuffer::create(device, &attachment_uniform, BufferUsages::UNIFORM);

        let terrain_bind_group = device.create_bind_group(
            "terrain_bind_group",
            &create_terrain_layout(device),
            &BindGroupEntries::sequential((
                &mesh_buffer,
                &terrain_config_buffer,
                &attachment_buffer,
                &atlas_sampler,
                &attachments[0],
                &attachments[1],
                &attachments[2],
                &attachments[3],
                &attachments[4],
                &attachments[5],
                &attachments[6],
                &attachments[7],
            )),
        );

        Self {
            mesh_buffer,
            terrain_bind_group,
        }
    }

    pub(crate) fn initialize(
        device: Res<RenderDevice>,
        fallback_image: Res<FallbackImage>,
        mut gpu_terrains: ResMut<TerrainComponents<GpuTerrain>>,
        gpu_tile_atlases: Res<TerrainComponents<GpuTileAtlas>>,
        tile_atlases: Extract<Query<(Entity, &TileAtlas), Added<TileAtlas>>>,
    ) {
        for (terrain, tile_atlas) in &tile_atlases {
            let gpu_tile_atlas = gpu_tile_atlases.get(&terrain).unwrap();

            gpu_terrains.insert(
                terrain,
                GpuTerrain::new(&device, &fallback_image, tile_atlas, gpu_tile_atlas),
            );
        }
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn extract(
        mut gpu_terrains: ResMut<TerrainComponents<GpuTerrain>>,
        terrains: Extract<
            Query<(Entity, &GlobalTransform, Option<&PreviousGlobalTransform>), With<TileAtlas>>,
        >,
    ) {
        for (terrain, transform, previous_transform) in terrains.iter() {
            let mesh_transforms = MeshTransforms {
                world_from_local: (&transform.affine()).into(),
                flags: 0,
                previous_world_from_local: (&previous_transform
                    .map(|t| t.0)
                    .unwrap_or(transform.affine()))
                    .into(),
            };
            let mesh_uniform = MeshUniform::new(&mesh_transforms, 0, None);

            let gpu_terrain = gpu_terrains.get_mut(&terrain).unwrap();
            gpu_terrain.mesh_buffer.set_value(mesh_uniform);
        }
    }

    pub(crate) fn prepare(
        queue: Res<RenderQueue>,
        mut gpu_terrains: ResMut<TerrainComponents<GpuTerrain>>,
    ) {
        for gpu_terrain in &mut gpu_terrains.values_mut() {
            gpu_terrain.mesh_buffer.update(&queue);
        }
    }
}

pub struct SetTerrainBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainBindGroup<I> {
    type Param = SRes<TerrainComponents<GpuTerrain>>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        _: Option<ROQueryItem<'w, Self::ItemQuery>>,
        gpu_terrains: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let gpu_terrain = gpu_terrains.into_inner().get(&item.entity()).unwrap();

        pass.set_bind_group(I, &gpu_terrain.terrain_bind_group, &[]);
        RenderCommandResult::Success
    }
}
