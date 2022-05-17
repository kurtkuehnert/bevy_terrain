use crate::{
    render::{resources::TerrainResources, PersistentComponents},
    GpuQuadtree, TerrainComputePipelines, TerrainConfig,
};
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
};

pub struct TerrainComputeData {
    pub(crate) prepare_node_list_count: usize,
    pub(crate) chunk_count: u32,
    pub(crate) indirect_buffer: Buffer,
    pub(crate) prepare_indirect_bind_group: BindGroup,
    pub(crate) build_node_list_bind_groups: [BindGroup; 2],
}

impl TerrainComputeData {
    fn new(
        device: &RenderDevice,
        config: &TerrainConfig,
        resources: &TerrainResources,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Self {
        let prepare_indirect_bind_group = Self::create_prepare_indirect_bind_group(
            device,
            resources,
            &compute_pipelines.prepare_indirect_layout,
        );
        let build_node_list_bind_groups = Self::create_build_node_list_bind_groups(
            device,
            resources,
            &compute_pipelines.build_node_list_layout,
        );

        Self {
            prepare_node_list_count: (config.lod_count - 1) as usize,
            chunk_count: config.chunk_count.x * config.chunk_count.y,
            indirect_buffer: resources.indirect_buffer.clone(),
            prepare_indirect_bind_group,
            build_node_list_bind_groups,
        }
    }

    fn create_build_node_list_bind_groups(
        device: &RenderDevice,
        resources: &TerrainResources,
        layout: &BindGroupLayout,
    ) -> [BindGroup; 2] {
        [
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: resources.config_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: resources.parameter_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: resources.temp_node_buffers[0].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: resources.temp_node_buffers[1].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: resources.final_node_buffer.as_entire_binding(),
                    },
                ],
                layout,
            }),
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: resources.config_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: resources.parameter_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: resources.temp_node_buffers[1].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: resources.temp_node_buffers[0].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: resources.final_node_buffer.as_entire_binding(),
                    },
                ],
                layout,
            }),
        ]
    }

    fn create_prepare_indirect_bind_group(
        device: &RenderDevice,
        resources: &TerrainResources,
        layout: &BindGroupLayout,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: resources.config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: resources.indirect_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: resources.parameter_buffer.as_entire_binding(),
                },
            ],
            layout,
        })
    }
}

/// Runs in queue.
pub(crate) fn initialize_terrain_compute_data(
    device: Res<RenderDevice>,
    compute_pipelines: Res<TerrainComputePipelines>,
    mut terrain_compute_data: ResMut<PersistentComponents<TerrainComputeData>>,
    terrain_query: Query<(Entity, &TerrainConfig, &TerrainResources)>,
) {
    for (entity, config, resources) in terrain_query.iter() {
        terrain_compute_data.insert(
            entity,
            TerrainComputeData::new(&device, &config, &resources, &compute_pipelines),
        );
    }
}
