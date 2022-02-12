use crate::node_atlas::NodeAtlas;
use crate::quadtree_update::{GpuQuadtreeUpdate, NodeUpdate};
use crate::terrain::TerrainConfig;
use crate::{TerrainComputePipeline, TerrainPipeline};
use bevy::ecs::system::lifetimeless::{Read, SQuery, SRes};
use bevy::ecs::system::SystemParamItem;
use bevy::reflect::TypeUuid;
use bevy::render::render_asset::{PrepareAssetError, RenderAsset, RenderAssets};
use bevy::render::render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self},
        render_resource::*,
        renderer::RenderContext,
    },
};
use bytemuck::cast_slice;
use std::mem;
use std::num::NonZeroU32;
use std::ops::Deref;

pub struct GpuPreparationData {
    pub(crate) lod_count: usize,
    pub(crate) quadtree_data: Vec<(BufferVec<NodeUpdate>, TextureView)>,
    pub(crate) indirect_buffer: Buffer,
    pub(crate) node_buffer_bind_groups: [BindGroup; 2],
    pub(crate) node_parameter_bind_group: BindGroup,
    pub(crate) patch_buffer_bind_group: BindGroup,
    pub(crate) patch_list_bind_group: BindGroup,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "c0172535-3220-4d02-ae1f-c35dcfde98c3"]
pub struct PreparationData {
    pub config: TerrainConfig,
}

impl PreparationData {
    fn create_quadtree_texture(&mut self, device: &RenderDevice, queue: &RenderQueue) -> Texture {
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

            queue.write_texture(texture, bytemuck::cast_slice(&data), data_layout, size);
        }

        quadtree_texture
    }

    fn create_node_buffers(&mut self, device: &RenderDevice) -> ([Buffer; 2], Buffer) {
        let max_node_count = self.config.chunk_count.x * self.config.chunk_count.y;

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: (max_node_count * mem::size_of::<u32>() as u32) as BufferAddress,
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
        let max_patch_count = self.config.chunk_count.x
            * self.config.chunk_count.y
            * self.config.patch_count
            * self.config.patch_count;

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: PATCH_SIZE * max_patch_count as BufferAddress,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        device.create_buffer(&buffer_descriptor)
    }
}

pub(crate) const INDIRECT_BUFFER_SIZE: BufferAddress = 5 * mem::size_of::<u32>() as BufferAddress;
pub(crate) const PARAMETER_BUFFER_SIZE: BufferAddress = 4 * mem::size_of::<u32>() as BufferAddress; // minimum buffer size = 16
pub(crate) const CONFIG_BUFFER_SIZE: BufferAddress = 4 * mem::size_of::<u32>() as BufferAddress;
pub(crate) const PATCH_SIZE: BufferAddress = 6 * mem::size_of::<u32>() as BufferAddress;

impl RenderAsset for PreparationData {
    type ExtractedAsset = PreparationData;
    type PreparedAsset = GpuPreparationData;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<TerrainPipeline>,
        SRes<TerrainComputePipeline>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        mut preparation: Self::ExtractedAsset,
        (device, queue, terrain_pipeline, compute_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let quadtree_texture = preparation.create_quadtree_texture(&device, &queue);

        let (node_temp_buffers, node_final_buffer) = preparation.create_node_buffers(device);

        let patch_buffer = preparation.create_patch_buffer(device);

        let config = &preparation.config;

        let config_uniform = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: cast_slice(&[
                config.area_count.x,
                config.area_count.y,
                config.lod_count,
                0,
            ]),
        });

        let indirect_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: INDIRECT_BUFFER_SIZE,
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        let parameter_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: PARAMETER_BUFFER_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

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

        let node_buffer_bind_groups = [
            device.create_bind_group(&BindGroupDescriptor {
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
                        resource: node_temp_buffers[0].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: node_temp_buffers[1].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: node_final_buffer.as_entire_binding(),
                    },
                ],
                label: None,
                layout: &compute_pipeline.node_buffer_bind_group_layout,
            }),
            device.create_bind_group(&BindGroupDescriptor {
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
                        resource: node_temp_buffers[1].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: node_temp_buffers[0].as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: node_final_buffer.as_entire_binding(),
                    },
                ],
                label: None,
                layout: &compute_pipeline.node_buffer_bind_group_layout,
            }),
        ];

        let node_parameter_bind_group = device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: indirect_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: parameter_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: config_uniform.as_entire_binding(),
                },
            ],
            label: None,
            layout: &compute_pipeline.node_parameter_bind_group_layout,
        });

        let quadtree_data = (0..config.lod_count)
            .map(|lod| {
                let mut buffer = BufferVec::default();
                buffer.reserve(1, device);

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
            .collect();

        let patch_buffer_bind_group = device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&quadtree_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: node_final_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: patch_buffer.as_entire_binding(),
                },
            ],
            label: None,
            layout: &compute_pipeline.patch_buffer_bind_group_layout,
        });

        let patch_list_bind_group = device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: patch_buffer.as_entire_binding(),
            }],
            label: None,
            layout: &terrain_pipeline.patch_list_layout,
        });

        Ok(GpuPreparationData {
            lod_count: config.lod_count as usize,
            quadtree_data,
            indirect_buffer,
            node_buffer_bind_groups,
            node_parameter_bind_group,
            patch_buffer_bind_group,
            patch_list_bind_group,
        })
    }
}
