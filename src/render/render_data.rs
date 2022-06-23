use crate::{
    render::{layouts::CONFIG_BUFFER_SIZE, resources::TerrainResources, PersistentComponents},
    GpuNodeAtlas, Terrain, TerrainConfig, TerrainRenderPipeline, TerrainView,
    TerrainViewComponents,
};
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
};

pub struct TerrainRenderData {
    pub(crate) terrain_data_bind_group: BindGroup,
}

impl TerrainRenderData {
    fn new(
        device: &RenderDevice,
        resources: &TerrainResources,
        gpu_node_atlas: &GpuNodeAtlas,
        terrain_pipeline: &mut TerrainRenderPipeline,
    ) -> Self {
        let terrain_data_layout = Self::create_terrain_data_layout(device, gpu_node_atlas);
        let terrain_data_bind_group = Self::create_terrain_data_bind_group(
            device,
            resources,
            gpu_node_atlas,
            &terrain_data_layout,
        );

        terrain_pipeline
            .terrain_data_layouts
            .push(terrain_data_layout);

        Self {
            terrain_data_bind_group,
        }
    }

    fn create_terrain_data_layout(
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
            label: None,
            entries: &entries,
        })
    }

    fn create_terrain_data_bind_group(
        device: &RenderDevice,
        resources: &TerrainResources,
        gpu_node_atlas: &GpuNodeAtlas,
        layout: &BindGroupLayout,
    ) -> BindGroup {
        let mut entries = vec![BindGroupEntry {
            binding: 0,
            resource: resources.config_buffer.as_entire_binding(),
        }];

        entries.extend(
            gpu_node_atlas
                .atlas_attachments
                .iter()
                .map(|(&binding, attachment)| attachment.bind_group_entry(binding)),
        );

        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &entries,
            layout,
        })
    }
}

/// Runs in queue.
pub(crate) fn initialize_terrain_render_data(
    device: Res<RenderDevice>,
    mut terrain_pipeline: ResMut<TerrainRenderPipeline>,
    gpu_node_atlases: Res<PersistentComponents<GpuNodeAtlas>>,
    terrain_resources: ResMut<TerrainViewComponents<TerrainResources>>,
    mut terrain_render_data: ResMut<PersistentComponents<TerrainRenderData>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<Entity, (With<Terrain>, With<TerrainConfig>)>,
) {
    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();
            let resources = terrain_resources.get(&(terrain, view)).unwrap();

            terrain_render_data.insert(
                terrain,
                TerrainRenderData::new(&device, &resources, gpu_node_atlas, &mut terrain_pipeline),
            );
        }
    }
}
