use crate::{
    render::layouts::CONFIG_BUFFER_SIZE,
    terrain::{Terrain, TerrainComponents},
    GpuNodeAtlas, TerrainComputePipelines, TerrainConfig, TerrainRenderPipeline,
};
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    render::{
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::RenderDevice,
        RenderWorld,
    },
};

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
    mut render_world: ResMut<RenderWorld>,
    device: Res<RenderDevice>,
    terrain_query: Query<(Entity, &TerrainConfig), Added<Terrain>>,
) {
    let mut terrain_data = render_world
        .remove_resource::<TerrainComponents<TerrainData>>()
        .unwrap();
    let gpu_node_atlases = render_world
        .remove_resource::<TerrainComponents<GpuNodeAtlas>>()
        .unwrap();
    let mut render_pipeline = render_world
        .remove_resource::<TerrainRenderPipeline>()
        .unwrap();
    let mut compute_pipelines = render_world.resource_mut::<TerrainComputePipelines>();

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

    render_world.insert_resource(render_pipeline);
    render_world.insert_resource(gpu_node_atlases);
    render_world.insert_resource(terrain_data);
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
