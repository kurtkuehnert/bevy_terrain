use crate::node_atlas::NodeUpdate;
use crate::quadtree::NodeData;
use crate::{
    config::TerrainConfig, node_atlas::NodeAtlas, render::layouts::*, TerrainComputePipelines,
    TerrainPipeline,
};
use bevy::core::cast_slice;
use bevy::utils::HashMap;
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{std140::Std140, *},
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
    },
};
use std::{mem, num::NonZeroU32, ops::Deref};

pub type PersistentComponent<A> = HashMap<Entity, A>;

#[derive(Component)]
pub struct InitTerrain;

pub enum NodeAttachment {}

#[derive(Component)]
pub struct TerrainResources {
    pub(crate) indirect_buffer: Option<Buffer>,
    pub(crate) parameter_buffer: Buffer,
    pub(crate) config_buffer: Buffer,
    pub(crate) temp_node_buffers: [Buffer; 2],
    pub(crate) final_node_buffer: Buffer,
    pub(crate) patch_buffer: Buffer,
    pub(crate) lod_map_view: TextureView,
    pub(crate) atlas_map_view: TextureView,
}

impl TerrainResources {
    pub(crate) fn new(config: &TerrainConfig, device: &RenderDevice) -> Self {
        let indirect_buffer = Some(Self::create_indirect_buffer(device));
        let parameter_buffer = Self::create_parameter_buffer(device);
        let config_buffer = Self::create_config_buffer(config, device);
        let (temp_node_buffers, final_node_buffer) = Self::create_node_buffers(config, device);
        let patch_buffer = Self::create_patch_buffer(config, device);
        let (lod_map_view, atlas_map_view) = Self::create_chunk_maps(config, device);

        Self {
            indirect_buffer,
            parameter_buffer,
            config_buffer,
            temp_node_buffers,
            final_node_buffer,
            patch_buffer,
            lod_map_view,
            atlas_map_view,
        }
    }

    fn create_indirect_buffer(device: &RenderDevice) -> Buffer {
        device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            contents: &[0; INDIRECT_BUFFER_SIZE as usize],
        })
    }

    fn create_config_buffer(config: &TerrainConfig, device: &RenderDevice) -> Buffer {
        device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: config.as_std140().as_bytes(),
        })
    }

    fn create_parameter_buffer(device: &RenderDevice) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: None,
            size: PARAMETER_BUFFER_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    fn create_node_buffers(config: &TerrainConfig, device: &RenderDevice) -> ([Buffer; 2], Buffer) {
        let max_node_count = config.chunk_count.x * config.chunk_count.y;

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: NODE_SIZE * max_node_count as BufferAddress,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        (
            [
                device.create_buffer(&buffer_descriptor),
                device.create_buffer(&buffer_descriptor),
            ],
            device.create_buffer(&buffer_descriptor),
        )
    }

    fn create_patch_buffer(config: &TerrainConfig, device: &RenderDevice) -> Buffer {
        let max_patch_count =
            config.chunk_count.x * config.chunk_count.y * TerrainConfig::PATCHES_PER_NODE;

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: PATCH_SIZE * max_patch_count as BufferAddress,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        device.create_buffer(&buffer_descriptor)
    }

    fn create_chunk_maps(
        config: &TerrainConfig,
        device: &RenderDevice,
    ) -> (TextureView, TextureView) {
        let chunk_count = config.chunk_count;

        let lod_map = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: chunk_count.x,
                height: chunk_count.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Uint,
            usage: TextureUsages::COPY_DST
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING,
        });

        let atlas_map = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: chunk_count.x,
                height: chunk_count.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Uint,
            usage: TextureUsages::COPY_DST
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING,
        });

        let lod_map_view = lod_map.create_view(&TextureViewDescriptor {
            label: None,
            format: Some(TextureFormat::R8Uint),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let atlas_map_view = atlas_map.create_view(&TextureViewDescriptor {
            label: None,
            format: Some(TextureFormat::R16Uint),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        (lod_map_view, atlas_map_view)
    }
}

pub struct GpuNodeAtlas {
    pub(crate) quadtree_view: TextureView,
    pub(crate) quadtree_update_buffers: Vec<Buffer>,
    pub(crate) quadtree_views: Vec<TextureView>,
    pub(crate) node_update_counts: Vec<u32>,
    quadtree_update: Vec<Vec<NodeUpdate>>,
    pub(crate) atlas_attachments: HashMap<String, NodeAttachment>,
    pub activated_nodes: Vec<(u16, NodeData)>, // make generic on NodeData
    pub(crate) height_atlas: GpuImage,
}

impl GpuNodeAtlas {
    fn new(config: &TerrainConfig, device: &RenderDevice, queue: &RenderQueue) -> Self {
        let (quadtree_view, quadtree_update_buffers, quadtree_views) =
            Self::create_quadtree(config, device, queue);

        let height_atlas = Self::create_node_atlas(config, device);

        Self {
            quadtree_view,
            quadtree_update_buffers,
            quadtree_views,
            node_update_counts: vec![],
            quadtree_update: vec![],
            atlas_attachments: Default::default(),
            activated_nodes: vec![],
            height_atlas,
        }
    }
    fn create_quadtree(
        config: &TerrainConfig,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) -> (TextureView, Vec<Buffer>, Vec<TextureView>) {
        let texture_descriptor = TextureDescriptor {
            label: None,
            size: Extent3d {
                width: config.chunk_count.x,
                height: config.chunk_count.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: config.lod_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Uint,
            usage: TextureUsages::COPY_DST
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING,
        };

        let quadtree_texture = device.create_texture(&texture_descriptor);

        // Todo: use https://docs.rs/wgpu/latest/wgpu/util/trait.DeviceExt.html#tymethod.create_buffer_init once its added to bevy

        for lod in 0..config.lod_count {
            let node_count = config.node_count(lod);

            let texture = ImageCopyTextureBase {
                texture: quadtree_texture.deref(),
                mip_level: lod,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            };

            let data_layout = ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::try_from(node_count.x * 2).unwrap()),
                rows_per_image: Some(NonZeroU32::try_from(node_count.y).unwrap()),
            };

            let size = Extent3d {
                width: node_count.x,
                height: node_count.y,
                depth_or_array_layers: 1,
            };

            let data: Vec<u16> =
                vec![NodeAtlas::INACTIVE_ID; (node_count.x * node_count.y) as usize];

            queue.write_texture(texture, cast_slice(&data), data_layout, size);
        }

        let quadtree_view = quadtree_texture.create_view(&TextureViewDescriptor {
            label: None,
            format: Some(TextureFormat::R16Uint),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let (quadtree_buffers, quadtree_views) = (0..config.lod_count)
            .map(|lod| {
                let node_count = config.node_count(lod);
                let max_node_count = (node_count.x * node_count.y) as BufferAddress;

                let buffer = device.create_buffer(&BufferDescriptor {
                    label: None,
                    size: NODE_UPDATE_SIZE * max_node_count,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let view = quadtree_texture.create_view(&TextureViewDescriptor {
                    label: None,
                    format: Some(TextureFormat::R16Uint),
                    dimension: Some(TextureViewDimension::D2),
                    aspect: TextureAspect::All,
                    base_mip_level: lod,
                    mip_level_count: NonZeroU32::new(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                });

                (buffer, view)
            })
            .unzip();

        (quadtree_view, quadtree_buffers, quadtree_views)
    }

    fn create_node_atlas(config: &TerrainConfig, device: &RenderDevice) -> GpuImage {
        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: config.texture_size,
                height: config.texture_size,
                depth_or_array_layers: config.node_atlas_size as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
            compare: None,
            anisotropy_clamp: None,
            border_color: None,
        });

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: None,
            format: None,
            dimension: Some(TextureViewDimension::D2Array),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        // Todo: consider using custom struct with only texture and view instead
        let height_atlas = GpuImage {
            texture,
            texture_view,
            texture_format: TextureFormat::R16Unorm,
            sampler,
            size: Size::new(config.texture_size as f32, config.texture_size as f32),
        };

        height_atlas
    }
}

pub struct TerrainBindGroups {
    pub(crate) indirect_buffer: Buffer,
    pub(crate) prepare_indirect_bind_group: BindGroup,
    pub(crate) update_quadtree_bind_groups: Vec<BindGroup>,
    pub(crate) build_node_list_bind_groups: [BindGroup; 2],
    pub(crate) build_patch_list_bind_group: BindGroup,
    pub(crate) build_chunk_maps_bind_group: BindGroup,
    pub(crate) terrain_data_bind_group: BindGroup,
    pub(crate) patch_list_bind_group: BindGroup,
}

impl TerrainBindGroups {
    pub(crate) fn new(
        resources: &mut TerrainResources,
        node_atlas: &GpuNodeAtlas,
        device: &RenderDevice,
        terrain_pipeline: &TerrainPipeline,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Self {
        let TerrainResources {
            ref mut indirect_buffer,
            ref parameter_buffer,
            ref config_buffer,
            ref temp_node_buffers,
            ref final_node_buffer,
            ref patch_buffer,
            ref lod_map_view,
            ref atlas_map_view,
        } = resources;

        let GpuNodeAtlas {
            ref quadtree_view,
            ref quadtree_update_buffers,
            ref quadtree_views,
            ref height_atlas,
            ..
        } = node_atlas;

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

        let terrain_data_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&atlas_map_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&height_atlas.texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&height_atlas.sampler),
                },
            ],
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

        let update_quadtree_bind_groups = quadtree_update_buffers
            .iter()
            .zip(quadtree_views.iter())
            .map(|(buffer, view)| {
                device.create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &compute_pipelines.update_quadtree_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: buffer.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();

        Self {
            indirect_buffer,
            prepare_indirect_bind_group,
            update_quadtree_bind_groups,
            build_node_list_bind_groups,
            build_patch_list_bind_group,
            build_chunk_maps_bind_group,
            terrain_data_bind_group,
            patch_list_bind_group,
        }
    }
}

/// Runs in extract.
pub(crate) fn notify_init_terrain(
    mut commands: Commands,
    terrain_query: Query<Entity, Changed<TerrainConfig>>,
) {
    for entity in terrain_query.iter() {
        commands.get_or_spawn(entity).insert(InitTerrain);
    }
}

/// Runs in prepare.
pub(crate) fn init_terrain_resources(
    mut commands: Commands,
    device: Res<RenderDevice>,
    terrain_query: Query<(Entity, &TerrainConfig), With<InitTerrain>>,
) {
    for (entity, config) in terrain_query.iter() {
        info!("initializing terrain resources");

        commands
            .get_or_spawn(entity)
            .insert(TerrainResources::new(config, &device));
    }
}

/// Runs in prepare.
pub(crate) fn init_node_atlas(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut gpu_node_atlases: ResMut<PersistentComponent<GpuNodeAtlas>>,
    terrain_query: Query<(Entity, &TerrainConfig), With<InitTerrain>>,
) {
    for (entity, config) in terrain_query.iter() {
        info!("initializing gpu node atlas");

        gpu_node_atlases.insert(entity, GpuNodeAtlas::new(config, &device, &queue));
    }
}

/// Runs in queue.
pub(crate) fn init_terrain_bind_groups(
    device: Res<RenderDevice>,
    terrain_pipeline: Res<TerrainPipeline>,
    compute_pipelines: Res<TerrainComputePipelines>,
    gpu_node_atlases: ResMut<PersistentComponent<GpuNodeAtlas>>,
    mut terrain_bind_groups: ResMut<PersistentComponent<TerrainBindGroups>>,
    mut terrain_query: Query<(Entity, &mut TerrainResources), With<InitTerrain>>,
) {
    for (entity, mut resources) in terrain_query.iter_mut() {
        info!("initializing terrain bind groups");

        let node_atlas = gpu_node_atlases.get(&entity).unwrap();

        terrain_bind_groups.insert(
            entity,
            TerrainBindGroups::new(
                &mut resources,
                &node_atlas,
                &device,
                &terrain_pipeline,
                &compute_pipelines,
            ),
        );
    }
}
