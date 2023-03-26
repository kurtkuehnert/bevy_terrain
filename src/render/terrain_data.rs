use crate::{
    render::TERRAIN_CONFIG_SIZE,
    terrain::{Terrain, TerrainComponents},
    TerrainConfig,
};
use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::RenderDevice,
        Extract,
    },
};
use std::num::NonZeroU8;

/// The terrain config data that is available in shaders.
#[derive(Clone, Default, ShaderType)]
pub(crate) struct TerrainConfigUniform {
    lod_count: u32,
    height: f32,
    chunk_size: u32,
    terrain_size: u32,
    attachment_sizes: Vec4,
    attachment_scales: Vec4,
    attachment_offsets: Vec4,
}

impl From<&TerrainConfig> for TerrainConfigUniform {
    fn from(config: &TerrainConfig) -> Self {
        // Todo: figure out a better way to store data for more than four attachments
        let mut sizes = [0.0; 4];
        let mut scales = [1.0; 4];
        let mut offsets = [0.0; 4];

        for (i, attachment) in config.attachments.iter().enumerate() {
            sizes[i] = attachment.texture_size as f32;
            scales[i] = attachment.center_size as f32 / attachment.texture_size as f32;
            offsets[i] = attachment.border_size as f32 / attachment.texture_size as f32;
        }

        Self {
            lod_count: config.lod_count,
            height: config.height,
            chunk_size: config.leaf_node_size,
            terrain_size: config.terrain_size,
            attachment_sizes: Vec4::from_array(sizes),
            attachment_scales: Vec4::from_array(scales),
            attachment_offsets: Vec4::from_array(offsets),
        }
    }
}

pub fn terrain_bind_group_layout(
    device: &RenderDevice,
    attachment_count: usize,
) -> BindGroupLayout {
    let mut entries = vec![
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::all(),
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(TERRAIN_CONFIG_SIZE),
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::all(),
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
    ];

    entries.extend((0..attachment_count).map(|binding| BindGroupLayoutEntry {
        binding: binding as u32 + 2,
        visibility: ShaderStages::all(),
        ty: BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: true },
            view_dimension: TextureViewDimension::D2Array,
            multisampled: false,
        },
        count: None,
    }));

    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: "terrain_layout".into(),
        entries: &entries,
    })
}

pub struct TerrainData {
    pub(crate) terrain_bind_group: BindGroup,
}

impl TerrainData {
    pub(crate) fn new(
        device: &RenderDevice,
        images: &RenderAssets<Image>,
        config: &TerrainConfig,
    ) -> Self {
        let layout = terrain_bind_group_layout(device, config.attachments.len());

        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(&TerrainConfigUniform::from(config)).unwrap();

        let config_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: &buffer.into_inner(),
        });

        let sampler_descriptor = SamplerDescriptor {
            address_mode_u: Default::default(),
            address_mode_v: Default::default(),
            address_mode_w: Default::default(),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            anisotropy_clamp: NonZeroU8::new(16),
            ..default()
        };

        let sampler = device.create_sampler(&sampler_descriptor);

        let mut entries = vec![
            BindGroupEntry {
                binding: 0,
                resource: config_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&sampler),
            },
        ];

        entries.extend(
            config
                .attachments
                .iter()
                .enumerate()
                .map(|(binding, attachment)| {
                    let attachment = images.get(&attachment.handle).unwrap();

                    BindGroupEntry {
                        binding: binding as u32 + 2,
                        resource: BindingResource::TextureView(&attachment.texture_view),
                    }
                }),
        );

        let terrain_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "terrain_bind_group".into(),
            entries: &entries,
            layout: &layout,
        });

        Self { terrain_bind_group }
    }
}

pub(crate) fn initialize_terrain_data(
    device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    mut terrain_data: ResMut<TerrainComponents<TerrainData>>,
    terrain_query: Extract<Query<(Entity, &TerrainConfig), Added<Terrain>>>,
) {
    for (terrain, config) in terrain_query.iter() {
        terrain_data.insert(terrain, TerrainData::new(&device, &images, config));
    }
}

pub struct SetTerrainBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainBindGroup<I> {
    type Param = SRes<TerrainComponents<TerrainData>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        terrain_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_data.into_inner().get(&item.entity()).unwrap();
        pass.set_bind_group(I, &data.terrain_bind_group, &[]);
        RenderCommandResult::Success
    }
}
