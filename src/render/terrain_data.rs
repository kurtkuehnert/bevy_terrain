use crate::{
    render::TERRAIN_CONFIG_SIZE,
    terrain::{Terrain, TerrainComponents},
    TerrainConfig,
};
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::RenderDevice,
        Extract,
    },
};

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
        let layout = terrain_bind_group_layout(&device, config.attachments.len());

        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(&config.shader_data()).unwrap();

        let config_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: &buffer.into_inner(),
        });

        let sampler_descriptor = SamplerDescriptor {
            label: None,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
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

impl<const I: usize> EntityRenderCommand for SetTerrainBindGroup<I> {
    type Param = SRes<TerrainComponents<TerrainData>>;

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        terrain_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_data.into_inner().get(&item).unwrap();
        pass.set_bind_group(I, &data.terrain_bind_group, &[]);
        RenderCommandResult::Success
    }
}
