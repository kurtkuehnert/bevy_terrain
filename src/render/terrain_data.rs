use crate::{
    config::TerrainConfig, node_atlas::NodeAtlas, render::layouts::*, TerrainComputePipelines,
    TerrainPipeline,
};
use bevy::core::cast_slice;
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
use std::{num::NonZeroU32, ops::Deref};

pub struct GpuTerrainData {
    pub(crate) config: TerrainConfig,
    pub(crate) indirect_buffer: Buffer,
    pub(crate) quadtree_update_buffers: Vec<Buffer>,
    pub(crate) prepare_indirect_bind_group: BindGroup,
    pub(crate) update_quadtree_bind_groups: Vec<BindGroup>,
    pub(crate) build_node_list_bind_groups: [BindGroup; 2],
    pub(crate) build_patch_list_bind_group: BindGroup,
    pub(crate) build_chunk_maps_bind_group: BindGroup,
    pub(crate) terrain_data_bind_group: BindGroup,
    pub(crate) patch_list_bind_group: BindGroup,
    pub(crate) height_atlas: GpuImage,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "32a1cd80-cef4-4534-b0ec-bc3a3d0800a9"]
pub struct TerrainData {
    pub(crate) config: TerrainConfig,
}

impl TerrainData {
    fn create_quadtree(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        layout: &BindGroupLayout,
    ) -> (TextureView, Vec<Buffer>, Vec<BindGroup>) {
        let config = &self.config;

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

        let (quadtree_buffers, quadtree_bind_groups) = (0..self.config.lod_count)
            .map(|lod| {
                let node_count = self.config.node_count(lod);
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

                let bind_group = device.create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: buffer.as_entire_binding(),
                        },
                    ],
                });

                (buffer, bind_group)
            })
            .unzip();

        (quadtree_view, quadtree_buffers, quadtree_bind_groups)
    }

    fn create_node_atlas(&mut self, device: &RenderDevice) -> GpuImage {
        let texture_size = self.config.texture_size;
        let node_atlas_size = self.config.node_atlas_size as u32; // array layers count

        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: node_atlas_size,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Uint,
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
            sampler,
            size: Size::new(texture_size as f32, texture_size as f32),
        };

        height_atlas
    }

    fn create_indirect_buffer(&mut self, device: &RenderDevice) -> Buffer {
        device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            contents: &[0; INDIRECT_BUFFER_SIZE as usize],
        })
    }

    fn create_config_buffer(&mut self, device: &RenderDevice) -> Buffer {
        device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: self.config.as_std140().as_bytes(),
        })
    }

    fn create_parameter_buffer(&mut self, device: &RenderDevice) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: None,
            size: PARAMETER_BUFFER_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    fn create_node_buffers(&mut self, device: &RenderDevice) -> ([Buffer; 2], Buffer) {
        let max_node_count = self.config.chunk_count.x * self.config.chunk_count.y;

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

    fn create_patch_buffer(&mut self, device: &RenderDevice) -> Buffer {
        let max_patch_count =
            self.config.chunk_count.x * self.config.chunk_count.y * TerrainConfig::PATCHES_PER_NODE;

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: PATCH_SIZE * max_patch_count as BufferAddress,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        device.create_buffer(&buffer_descriptor)
    }

    fn create_chunk_maps(&mut self, device: &RenderDevice) -> (TextureView, TextureView) {
        let chunk_count = self.config.chunk_count;

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

impl RenderAsset for TerrainData {
    type ExtractedAsset = TerrainData;
    type PreparedAsset = GpuTerrainData;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<TerrainPipeline>,
        SRes<TerrainComputePipelines>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        mut terrain_data: Self::ExtractedAsset,
        (device, queue, terrain_pipeline, compute_pipelines): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        info!("initializing terrain data");

        let (quadtree_view, quadtree_update_buffers, update_quadtree_bind_groups) =
            terrain_data.create_quadtree(device, queue, &compute_pipelines.update_quadtree_layout);
        let height_atlas = terrain_data.create_node_atlas(device);

        let indirect_buffer = terrain_data.create_indirect_buffer(device);
        let config_buffer = terrain_data.create_config_buffer(device);
        let parameter_buffer = terrain_data.create_parameter_buffer(device);
        let (temp_node_buffers, final_node_buffer) = terrain_data.create_node_buffers(device);
        let patch_buffer = terrain_data.create_patch_buffer(device);

        let (lod_map_view, atlas_map_view) = terrain_data.create_chunk_maps(device);

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
                    resource: BindingResource::TextureView(&height_atlas.texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&atlas_map_view),
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

        Ok(GpuTerrainData {
            config: terrain_data.config,
            indirect_buffer,
            quadtree_update_buffers,
            prepare_indirect_bind_group,
            update_quadtree_bind_groups,
            build_node_list_bind_groups,
            build_patch_list_bind_group,
            build_chunk_maps_bind_group,
            terrain_data_bind_group,
            patch_list_bind_group,
            height_atlas,
        })
    }
}
