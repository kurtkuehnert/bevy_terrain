use crate::{
    render::layouts::CONFIG_BUFFER_SIZE,
    terrain::{Terrain, TerrainComponents},
    GpuNodeAtlas, TerrainConfig, TerrainRenderPipeline,
};
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice, RenderWorld},
};

pub struct TerrainData {
    pub(crate) terrain_bind_group: BindGroup,
}

impl TerrainData {
    fn new(
        device: &RenderDevice,
        config: &TerrainConfig,
        gpu_node_atlas: &GpuNodeAtlas,
        terrain_pipeline: &mut TerrainRenderPipeline,
    ) -> Self {
        let config_buffer = Self::create_config_buffer(device, config);

        let terrain_layout = Self::create_terrain_layout(device, gpu_node_atlas);
        let terrain_bind_group = Self::create_terrain_bind_group(
            device,
            &config_buffer,
            gpu_node_atlas,
            &terrain_layout,
        );

        terrain_pipeline.terrain_layouts.push(terrain_layout);

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
                visibility: ShaderStages::VERTEX_FRAGMENT,
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
    let mut terrain_pipeline = render_world.resource_mut::<TerrainRenderPipeline>();

    for (terrain, config) in terrain_query.iter() {
        let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();

        terrain_data.insert(
            terrain,
            TerrainData::new(&device, config, gpu_node_atlas, &mut terrain_pipeline),
        );
    }

    render_world.insert_resource(gpu_node_atlases);
    render_world.insert_resource(terrain_data);
}
