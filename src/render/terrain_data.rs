use crate::{
    render::layouts::CONFIG_BUFFER_SIZE,
    terrain::{Terrain, TerrainComponents},
    GpuNodeAtlas, TerrainComputePipelines, TerrainConfig, TerrainRenderPipeline,
};
use bevy::render::render_asset::RenderAssets;
use bevy::render::texture::FallbackImage;
use bevy::render::Extract;
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    render::{
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::RenderDevice,
    },
};

// Todo: create in setup, extract once added, prepare into terrain data
pub struct TerrainMaterial<const COUNT: usize> {
    pub config: TerrainConfig,
    pub attachments: [Handle<Image>; COUNT],
}

impl<const COUNT: usize> AsBindGroup for TerrainMaterial<COUNT> {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &RenderAssets<Image>,
        _fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self>, AsBindGroupError> {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(&self.config.shader_data()).unwrap();

        let config_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: "config_buffer".into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: &buffer.into_inner(),
        });

        let sampler_descriptor = SamplerDescriptor {
            label: "default_sampler_attachment".into(),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            ..default()
        };

        let sampler = render_device.create_sampler(&sampler_descriptor);

        let mut bindings = vec![
            OwnedBindingResource::Buffer(config_buffer.clone()),
            OwnedBindingResource::Sampler(sampler.clone()),
        ];

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

        for (binding, handle) in self.attachments.iter().enumerate() {
            let attachment = images.get(handle).unwrap();

            bindings.push(OwnedBindingResource::TextureView(
                attachment.texture_view.clone(),
            ));

            entries.push(BindGroupEntry {
                binding: binding as u32 + 2,
                resource: BindingResource::TextureView(&attachment.texture_view),
            });
        }

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: "terrain_bind_group".into(),
            entries: &entries,
            layout,
        });

        Ok(PreparedBindGroup {
            bindings,
            bind_group,
            data: (),
        })
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        let mut entries = vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::all(),
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(CONFIG_BUFFER_SIZE),
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

        entries.extend((0..COUNT).map(|binding| BindGroupLayoutEntry {
            binding: binding as u32 + 2,
            visibility: ShaderStages::all(),
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        }));

        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "terrain_layout".into(),
            entries: &entries,
        })
    }
}

pub struct TerrainData {
    pub(crate) terrain_bind_group: BindGroup,
}

impl TerrainData {
    fn new(
        device: &RenderDevice,
        config: &TerrainConfig,
        gpu_node_atlas: &GpuNodeAtlas,
        render_pipeline: &mut TerrainRenderPipeline,
        compute_pipelines: &mut TerrainComputePipelines,
    ) -> Self {
        let config_buffer = Self::create_config_buffer(device, config);

        let terrain_layout = Self::create_terrain_layout(device, gpu_node_atlas);
        let terrain_bind_group = Self::create_terrain_bind_group(
            device,
            &config_buffer,
            gpu_node_atlas,
            &terrain_layout,
        );

        render_pipeline.terrain_layouts.push(terrain_layout.clone());
        compute_pipelines.terrain_layouts.push(terrain_layout);

        Self { terrain_bind_group }
    }

    fn create_config_buffer(device: &RenderDevice, config: &TerrainConfig) -> Buffer {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(&config.shader_data()).unwrap();

        device.create_buffer_with_data(&BufferInitDescriptor {
            label: "config_buffer".into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: &buffer.into_inner(),
        })
    }

    fn create_terrain_layout(
        device: &RenderDevice,
        gpu_node_atlas: &GpuNodeAtlas,
    ) -> BindGroupLayout {
        let mut entries = vec![
            // config buffer
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::all(),
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(CONFIG_BUFFER_SIZE),
                },
                count: None,
            },
        ];

        entries.extend(
            gpu_node_atlas
                .atlas_attachments
                .iter()
                .map(|(&binding, attachment)| attachment.layout_entry(binding)),
        );

        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "terrain_layout".into(),
            entries: &entries,
        })
    }

    fn create_terrain_bind_group(
        device: &RenderDevice,
        config_buffer: &Buffer,
        gpu_node_atlas: &GpuNodeAtlas,
        layout: &BindGroupLayout,
    ) -> BindGroup {
        let mut entries = vec![BindGroupEntry {
            binding: 0,
            resource: config_buffer.as_entire_binding(),
        }];

        entries.extend(
            gpu_node_atlas
                .atlas_attachments
                .iter()
                .map(|(&binding, attachment)| attachment.bind_group_entry(binding)),
        );

        device.create_bind_group(&BindGroupDescriptor {
            label: "terrain_bind_group".into(),
            entries: &entries,
            layout,
        })
    }
}

pub(crate) fn initialize_terrain_data(
    device: Res<RenderDevice>,
    mut render_pipeline: ResMut<TerrainRenderPipeline>,
    mut compute_pipelines: ResMut<TerrainComputePipelines>,
    mut terrain_data: ResMut<TerrainComponents<TerrainData>>,
    gpu_node_atlases: Res<TerrainComponents<GpuNodeAtlas>>,
    terrain_query: Extract<Query<(Entity, &TerrainConfig), Added<Terrain>>>,
) {
    for (terrain, config) in terrain_query.iter() {
        let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();

        terrain_data.insert(
            terrain,
            TerrainData::new(
                &device,
                config,
                gpu_node_atlas,
                &mut render_pipeline,
                &mut compute_pipelines,
            ),
        );
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
