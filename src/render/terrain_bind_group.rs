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
        renderer::RenderDevice,
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
}

impl From<&TerrainConfig> for TerrainConfigUniform {
    fn from(config: &TerrainConfig) -> Self {
        Self {
            lod_count: config.lod_count,
            min_height: config.min_height,
            max_height: config.max_height,
        }
    }
}

pub struct TerrainData {
    pub(crate) terrain_bind_group: BindGroup,
}

impl TerrainData {
    fn new(
        device: &RenderDevice,
        fallback_image: &FallbackImage,
        config_uniform: TerrainConfigUniform,
        mesh_uniform: MeshUniform,
        gpu_node_atlas: &GpuNodeAtlas,
    ) -> Self {
        let mesh_buffer = StaticBuffer::create(device, &mesh_uniform, BufferUsages::STORAGE);
        let terrain_config_buffer =
            StaticBuffer::create(device, &config_uniform, BufferUsages::UNIFORM);

        let atlas_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
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
            StaticBuffer::create(device, &attachment_uniform, BufferUsages::UNIFORM);

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

        Self { terrain_bind_group }
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn initialize(
        device: Res<RenderDevice>,
        fallback_image: Res<FallbackImage>,
        mut terrain_data: ResMut<TerrainComponents<TerrainData>>,
        gpu_node_atlases: Res<TerrainComponents<GpuNodeAtlas>>,
        terrain_query: Extract<
            Query<
                (
                    Entity,
                    &TerrainConfig,
                    &GlobalTransform,
                    Option<&PreviousGlobalTransform>,
                ),
                Added<Terrain>,
            >,
        >,
    ) {
        for (terrain, config, transform, previous_transform) in terrain_query.iter() {
            // Todo: update the transform each frame

            let transform = transform.affine();
            let previous_transform = previous_transform.map(|t| t.0).unwrap_or(transform);
            let mesh_transforms = MeshTransforms {
                transform: (&transform).into(),
                previous_transform: (&previous_transform).into(),
                flags: 0,
            };
            let (inverse_transpose_model_a, inverse_transpose_model_b) =
                mesh_transforms.transform.inverse_transpose_3x3();
            let mesh_uniform = MeshUniform {
                transform: mesh_transforms.transform.to_transpose(),
                previous_transform: mesh_transforms.previous_transform.to_transpose(),
                lightmap_uv_rect: UVec2::ZERO,
                inverse_transpose_model_a,
                inverse_transpose_model_b,
                flags: mesh_transforms.flags,
            };
            let config_uniform = config.into();

            let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();

            terrain_data.insert(
                terrain,
                TerrainData::new(
                    &device,
                    &fallback_image,
                    config_uniform,
                    mesh_uniform,
                    gpu_node_atlas,
                ),
            );
        }
    }
}

pub struct SetTerrainBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainBindGroup<I> {
    type Param = SRes<TerrainComponents<TerrainData>>;
    type ViewData = ();
    type ItemData = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _: ROQueryItem<'w, Self::ViewData>,
        _: ROQueryItem<'w, Self::ItemData>,
        terrain_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_data.into_inner().get(&item.entity()).unwrap();

        pass.set_bind_group(I, &data.terrain_bind_group, &[]);
        RenderCommandResult::Success
    }
}
