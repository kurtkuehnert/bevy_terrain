use crate::{
    attachments::NodeAttachment, render::resources::TerrainResources, GpuNodeAtlas, GpuQuadtree,
    PersistentComponents, TerrainComputePipelines, TerrainConfig, TerrainRenderPipeline,
};
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
};
use std::mem;

pub struct TerrainBindGroups {
    pub(crate) prepare_node_list_count: usize,
    pub(crate) chunk_count: u32,
    pub(crate) indirect_buffer: Buffer,
    pub(crate) prepare_indirect_bind_group: BindGroup,
    pub(crate) build_node_list_bind_groups: [BindGroup; 2],
    pub(crate) build_patch_list_bind_group: BindGroup,
    pub(crate) build_chunk_maps_bind_group: BindGroup,
    pub(crate) terrain_data_bind_group: BindGroup, // Todo: factor out?
    pub(crate) patch_list_bind_group: BindGroup,
}

impl TerrainBindGroups {
    pub(crate) fn new(
        resources: &mut TerrainResources,
        gpu_quadtree: &GpuQuadtree,
        gpu_node_atlas: &GpuNodeAtlas,
        device: &RenderDevice,
        terrain_pipeline: &TerrainRenderPipeline,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Self {
        let TerrainResources {
            prepare_node_list_count,
            chunk_count,
            ref mut indirect_buffer,
            ref parameter_buffer,
            ref config_buffer,
            ref temp_node_buffers,
            ref final_node_buffer,
            ref patch_buffer,
            ref lod_map_view,
            ref atlas_map_view,
        } = resources;

        let GpuQuadtree {
            view: ref quadtree_view,
            ..
        } = gpu_quadtree;

        let indirect_buffer = mem::take(indirect_buffer).unwrap();

        let prepare_indirect_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: indirect_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: parameter_buffer.as_entire_binding(),
                },
            ],
            layout: &compute_pipelines.prepare_indirect_layout,
        });

        let build_node_list_bind_groups = [
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&quadtree_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: parameter_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: temp_node_buffers[0].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: temp_node_buffers[1].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: final_node_buffer.as_entire_binding(),
                    },
                ],
                layout: &compute_pipelines.build_node_list_layout,
            }),
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&quadtree_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: parameter_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: temp_node_buffers[1].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: temp_node_buffers[0].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: final_node_buffer.as_entire_binding(),
                    },
                ],
                layout: &compute_pipelines.build_node_list_layout,
            }),
        ];

        let build_patch_list_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&quadtree_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: parameter_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: final_node_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: patch_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&lod_map_view),
                },
            ],
            layout: &compute_pipelines.build_patch_list_layout,
        });

        let build_chunk_maps_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&quadtree_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: parameter_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: final_node_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&lod_map_view),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&atlas_map_view),
                },
            ],
            layout: &compute_pipelines.build_chunk_maps_layout,
        });

        let mut entries = vec![
            BindGroupEntry {
                binding: 0,
                resource: config_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(&atlas_map_view),
            },
        ];

        for attachment in gpu_node_atlas.atlas_attachments.values() {
            match attachment {
                &NodeAttachment::Buffer {
                    binding,
                    ref buffer,
                } => entries.push(BindGroupEntry {
                    binding,
                    resource: buffer.as_entire_binding(),
                }),
                &NodeAttachment::Texture {
                    view_binding,
                    sampler_binding,
                    ref view,
                    ref sampler,
                    ..
                } => {
                    entries.push(BindGroupEntry {
                        binding: view_binding,
                        resource: BindingResource::TextureView(view),
                    });
                    entries.push(BindGroupEntry {
                        binding: sampler_binding,
                        resource: BindingResource::Sampler(sampler),
                    });
                }
            }
        }

        let terrain_data_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &entries,
            layout: &terrain_pipeline.terrain_data_layout,
        });

        let patch_list_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: patch_buffer.as_entire_binding(),
            }],
            layout: &terrain_pipeline.patch_list_layout,
        });

        Self {
            prepare_node_list_count: *prepare_node_list_count,
            chunk_count: *chunk_count,
            indirect_buffer,
            prepare_indirect_bind_group,
            build_node_list_bind_groups,
            build_patch_list_bind_group,
            build_chunk_maps_bind_group,
            terrain_data_bind_group,
            patch_list_bind_group,
        }
    }
}

/// Runs in queue.
pub(crate) fn init_terrain_bind_groups(
    device: Res<RenderDevice>,
    terrain_pipeline: Res<TerrainRenderPipeline>,
    compute_pipelines: Res<TerrainComputePipelines>,
    gpu_quadtrees: Res<PersistentComponents<GpuQuadtree>>,
    gpu_node_atlases: Res<PersistentComponents<GpuNodeAtlas>>,
    mut terrain_bind_groups: ResMut<PersistentComponents<TerrainBindGroups>>,
    mut terrain_query: Query<(Entity, &mut TerrainResources), With<TerrainConfig>>,
) {
    for (entity, mut resources) in terrain_query.iter_mut() {
        info!("initializing terrain bind groups");

        let gpu_quadtree = gpu_quadtrees.get(&entity).unwrap();
        let gpu_node_atlas = gpu_node_atlases.get(&entity).unwrap();

        terrain_bind_groups.insert(
            entity,
            TerrainBindGroups::new(
                &mut resources,
                gpu_quadtree,
                gpu_node_atlas,
                &device,
                &terrain_pipeline,
                &compute_pipelines,
            ),
        );
    }
}
