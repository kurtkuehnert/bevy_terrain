use crate::{
    terrain::TerrainComponents,
    terrain_data::{GpuTileAtlas, TileAtlas},
};
use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    math::Affine3,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::RenderDevice,
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
        texture::FallbackImage,
        Extract,
    },
};
use itertools::Itertools;
use std::iter;

// Todo: use this once texture views can be used directly
#[derive(AsBindGroup)]
pub struct TerrainBindGroup {
    #[storage(0, visibility(all), read_only, buffer)]
    terrain: Buffer,
    #[uniform(1, visibility(all))]
    attachments: AttachmentUniform,
    #[sampler(2, visibility(all))]
    #[texture(3, visibility(all), dimension = "2d_array")]
    attachment0: Handle<Image>,
    #[texture(4, visibility(all), dimension = "2d_array")]
    attachment1: Handle<Image>,
    #[texture(5, visibility(all), dimension = "2d_array")]
    attachment2: Handle<Image>,
    #[texture(6, visibility(all), dimension = "2d_array")]
    attachment3: Handle<Image>,
    #[texture(7, visibility(all), dimension = "2d_array")]
    attachment4: Handle<Image>,
    #[texture(8, visibility(all), dimension = "2d_array")]
    attachment5: Handle<Image>,
    #[texture(9, visibility(all), dimension = "2d_array")]
    attachment6: Handle<Image>,
    #[texture(10, visibility(all), dimension = "2d_array")]
    attachment7: Handle<Image>,
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
pub struct TerrainUniform {
    lod_count: u32,
    scale: f32,
    world_from_local: [Vec4; 3],
    local_from_world_transpose_a: [Vec4; 2],
    local_from_world_transpose_b: f32,
}

impl TerrainUniform {
    pub fn new(tile_atlas: &TileAtlas, global_transform: &GlobalTransform) -> Self {
        let transform = Affine3::from(&global_transform.affine());
        let world_from_local = transform.to_transpose();
        let (local_from_world_transpose_a, local_from_world_transpose_b) =
            transform.inverse_transpose_3x3();

        Self {
            lod_count: tile_atlas.lod_count,
            scale: tile_atlas.model.scale() as f32,
            world_from_local,
            local_from_world_transpose_a,
            local_from_world_transpose_b,
        }
    }
}

pub struct GpuTerrain {
    pub(crate) terrain_bind_group: Option<BindGroup>,

    terrain_buffer: Handle<ShaderStorageBuffer>,
    attachment_buffer: Buffer,
    atlas_sampler: Sampler,
    attachments: Vec<TextureView>,
}

impl GpuTerrain {
    fn new(
        device: &RenderDevice,
        fallback_image: &FallbackImage,
        tile_atlas: &TileAtlas,
        gpu_tile_atlas: &GpuTileAtlas,
    ) -> Self {
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

        let value = AttachmentUniform::new(gpu_tile_atlas);
        let mut buffer = vec![0; value.size().get() as usize];
        encase::UniformBuffer::new(&mut buffer)
            .write(&value)
            .unwrap();

        let attachment_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            contents: &buffer,
            usage: BufferUsages::UNIFORM,
        });

        Self {
            terrain_buffer: tile_atlas.terrain_buffer.clone(),
            attachment_buffer,
            atlas_sampler,
            attachments,
            terrain_bind_group: None,
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

    pub(crate) fn prepare(
        device: Res<RenderDevice>,
        buffers: Res<RenderAssets<GpuShaderStorageBuffer>>,
        mut gpu_terrains: ResMut<TerrainComponents<GpuTerrain>>,
    ) {
        for gpu_terrain in &mut gpu_terrains.values_mut() {
            let terrain_buffer = buffers.get(&gpu_terrain.terrain_buffer).unwrap();

            // Todo: be smarter about bind group recreation
            gpu_terrain.terrain_bind_group = Some(device.create_bind_group(
                "terrain_bind_group",
                &TerrainBindGroup::bind_group_layout(&device),
                &BindGroupEntries::sequential((
                    terrain_buffer.buffer.as_entire_binding(),
                    gpu_terrain.attachment_buffer.as_entire_binding(),
                    &gpu_terrain.atlas_sampler,
                    &gpu_terrain.attachments[0],
                    &gpu_terrain.attachments[1],
                    &gpu_terrain.attachments[2],
                    &gpu_terrain.attachments[3],
                    &gpu_terrain.attachments[4],
                    &gpu_terrain.attachments[5],
                    &gpu_terrain.attachments[6],
                    &gpu_terrain.attachments[7],
                )),
            ));
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

        if let Some(bind_group) = &gpu_terrain.terrain_bind_group {
            pass.set_bind_group(I, bind_group, &[]);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Skip
        }
    }
}
