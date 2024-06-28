use crate::{
    terrain::{Terrain, TerrainComponents, TerrainConfig},
    terrain_data::gpu_node_atlas::GpuNodeAtlas,
    util::StaticBuffer,
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
                uniform_buffer::<TerrainConfigUniform>(false),  // terrain config
                uniform_buffer::<AttachmentUniform>(false),
                sampler(SamplerBindingType::Filtering), // atlas sampler
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment 1
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment 2
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment 3
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment 4
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment 5
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment 6
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment 7
                texture_2d_array(TextureSampleType::Float { filterable: true }), // attachment 8
            ),
        ),
    )
}

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
    fn new(atlas: &GpuNodeAtlas) -> Self {
        let mut uniform = Self::default();

        for (config, attachment) in iter::zip(&mut uniform.data, &atlas.attachments) {
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
    min_height: f32,
    max_height: f32,
    scale: f32,
}

impl From<&TerrainConfig> for TerrainConfigUniform {
    fn from(config: &TerrainConfig) -> Self {
        Self {
            lod_count: config.lod_count,
            min_height: config.min_height,
            max_height: config.max_height,
            scale: config.model.radius() as f32,
        }
    }
}

pub struct TerrainData {
    mesh_buffer: StaticBuffer<MeshUniform>,
    pub(crate) terrain_bind_group: BindGroup,
}

impl TerrainData {
    fn new(
        device: &RenderDevice,
        fallback_image: &FallbackImage,
        config_uniform: TerrainConfigUniform,
        gpu_node_atlas: &GpuNodeAtlas,
    ) -> Self {
        let mesh_buffer = StaticBuffer::empty_sized(
            None,
            device,
            MeshUniform::SHADER_SIZE.get(),
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let terrain_config_buffer =
            StaticBuffer::create(None, device, &config_uniform, BufferUsages::UNIFORM);

        let atlas_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            anisotropy_clamp: 16, // Todo: make this customisable
            ..default()
        });

        let attachments = (0..8)
            .map(|i| {
                gpu_node_atlas
                    .attachments
                    .get(i)
                    .map_or(fallback_image.d2_array.texture_view.clone(), |attachment| {
                        attachment.atlas_texture.create_view(&default())
                    })
            })
            .collect_vec();

        let attachment_uniform = AttachmentUniform::new(gpu_node_atlas);
        let attachment_buffer =
            StaticBuffer::create(None, device, &attachment_uniform, BufferUsages::UNIFORM);

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
        mut terrain_data: ResMut<TerrainComponents<TerrainData>>,
        gpu_node_atlases: Res<TerrainComponents<GpuNodeAtlas>>,
        terrain_query: Extract<Query<(Entity, &TerrainConfig), Added<Terrain>>>,
    ) {
        for (terrain, config) in terrain_query.iter() {
            let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();

            terrain_data.insert(
                terrain,
                TerrainData::new(&device, &fallback_image, config.into(), gpu_node_atlas),
            );
        }
    }

    pub(crate) fn extract(
        mut terrain_data: ResMut<TerrainComponents<TerrainData>>,
        terrain_query: Extract<
            Query<(Entity, &GlobalTransform, Option<&PreviousGlobalTransform>), With<Terrain>>,
        >,
    ) {
        for (terrain, transform, previous_transform) in terrain_query.iter() {
            let mesh_transforms = MeshTransforms {
                world_from_local: (&transform.affine()).into(),
                flags: 0,
                previous_world_from_local: (&previous_transform
                    .map(|t| t.0)
                    .unwrap_or(transform.affine()))
                    .into(),
            };
            let mesh_uniform = MeshUniform::new(&mesh_transforms, None);

            let terrain_data = terrain_data.get_mut(&terrain).unwrap();
            terrain_data.mesh_buffer.set_value(mesh_uniform);
        }
    }

    pub(crate) fn prepare(
        queue: Res<RenderQueue>,
        mut terrain_data: ResMut<TerrainComponents<TerrainData>>,
    ) {
        for terrain_data in &mut terrain_data.values_mut() {
            terrain_data.mesh_buffer.update(&queue);
        }
    }
}

pub struct SetTerrainBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainBindGroup<I> {
    type Param = SRes<TerrainComponents<TerrainData>>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        _: Option<ROQueryItem<'w, Self::ItemQuery>>,
        terrain_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_data.into_inner().get(&item.entity()).unwrap();

        pass.set_bind_group(I, &data.terrain_bind_group, &[]);
        RenderCommandResult::Success
    }
}
