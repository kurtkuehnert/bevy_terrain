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
    pub(crate) build_patch_list_bind_group: BindGroup,
    pub(crate) build_chunk_maps_bind_group: BindGroup,
}

impl TerrainComputeData {
    fn new(
        device: &RenderDevice,
        config: &TerrainConfig,
        resources: &TerrainResources,
        gpu_quadtree: &GpuQuadtree,
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
            gpu_quadtree,
            &compute_pipelines.build_node_list_layout,
        );
        let build_patch_list_bind_group = Self::create_build_patch_list_bind_group(
            device,
            resources,
            gpu_quadtree,
            &compute_pipelines.build_patch_list_layout,
        );
        let build_chunk_maps_bind_group = Self::create_build_chunk_maps_bind_group(
            device,
            resources,
            gpu_quadtree,
            &compute_pipelines.build_chunk_maps_layout,
        );

        Self {
            prepare_node_list_count: (config.lod_count - 1) as usize,
            chunk_count: config.chunk_count.x * config.chunk_count.y,
            indirect_buffer: resources.indirect_buffer.clone(),
            prepare_indirect_bind_group,
            build_node_list_bind_groups,
            build_patch_list_bind_group,
            build_chunk_maps_bind_group,
        }
    }

    fn create_build_patch_list_bind_group(
        device: &RenderDevice,
        resources: &TerrainResources,
        gpu_quadtree: &GpuQuadtree,
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
                    resource: BindingResource::TextureView(&gpu_quadtree.view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: resources.parameter_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: resources.final_node_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: resources.patch_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&resources.lod_map_view),
                },
            ],
            layout,
        })
    }

    fn create_build_node_list_bind_groups(
        device: &RenderDevice,
        resources: &TerrainResources,
        gpu_quadtree: &GpuQuadtree,
        layout: &BindGroupLayout,
    ) -> [BindGroup; 2] {
        [
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&gpu_quadtree.view),
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
                        resource: BindingResource::TextureView(&gpu_quadtree.view),
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

    fn create_build_chunk_maps_bind_group(
        device: &RenderDevice,
        resources: &TerrainResources,
        gpu_quadtree: &GpuQuadtree,
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
                    resource: BindingResource::TextureView(&gpu_quadtree.view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: resources.parameter_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: resources.final_node_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&resources.lod_map_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&resources.atlas_map_view),
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
    gpu_quadtrees: Res<PersistentComponents<GpuQuadtree>>,
    mut terrain_compute_data: ResMut<PersistentComponents<TerrainComputeData>>,
    terrain_query: Query<(Entity, &TerrainConfig, &TerrainResources)>,
) {
    for (entity, config, resources) in terrain_query.iter() {
        let gpu_quadtree = gpu_quadtrees.get(&entity).unwrap();

        terrain_compute_data.insert(
            entity,
            TerrainComputeData::new(
                &device,
                &config,
                &resources,
                gpu_quadtree,
                &compute_pipelines,
            ),
        );
    }
}
