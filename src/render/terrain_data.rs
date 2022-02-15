use crate::{
    node_atlas::NodeAtlas,
    quadtree_update::NodeUpdate,
    terrain::{TerrainConfig, TerrainConfigUniform},
    TerrainComputePipeline, TerrainPipeline,
};
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{
            std140::{AsStd140, Std140},
            *,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
    },
};
use bytemuck::cast_slice;
use std::{mem, num::NonZeroU32, ops::Deref};

pub(crate) const INDIRECT_BUFFER_SIZE: BufferAddress = 5 * mem::size_of::<u32>() as BufferAddress;
pub(crate) const PARAMETER_BUFFER_SIZE: BufferAddress = 4 * mem::size_of::<u32>() as BufferAddress; // minimum buffer size = 16
pub(crate) const CONFIG_BUFFER_SIZE: BufferAddress = 4 * mem::size_of::<u32>() as BufferAddress;
pub(crate) const PATCH_SIZE: BufferAddress = 6 * mem::size_of::<u32>() as BufferAddress;

pub struct GpuTerrainData {
    pub(crate) config: TerrainConfig,
    pub(crate) quadtree_data: Vec<(BufferVec<NodeUpdate>, TextureView)>,
    pub(crate) indirect_buffer: Buffer,
    pub(crate) node_buffer_bind_groups: [BindGroup; 2],
    pub(crate) node_parameter_bind_group: BindGroup,
    pub(crate) patch_buffer_bind_group: BindGroup,
    pub(crate) patch_list_bind_group: BindGroup,
    pub(crate) terrain_data_bind_group: BindGroup,
    pub(crate) height_atlas: GpuImage,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "32a1cd80-cef4-4534-b0ec-bc3a3d0800a9"]
pub struct TerrainData {
    pub(crate) config: TerrainConfig,
}

impl TerrainData {
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

        let height_atlas = GpuImage {
            texture,
            texture_view,
            sampler,
            size: Size::new(texture_size as f32, texture_size as f32),
        };

        height_atlas
    }
}

impl RenderAsset for TerrainData {
    type ExtractedAsset = TerrainData;
    type PreparedAsset = GpuTerrainData;
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
        mut terrain_data: Self::ExtractedAsset,
        (device, queue, terrain_pipeline, compute_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        info!("initializing terrain data");

        let quadtree_texture = terrain_data.create_quadtree_texture(&device, &queue);

        let (node_temp_buffers, node_final_buffer) = terrain_data.create_node_buffers(device);

        let patch_buffer = terrain_data.create_patch_buffer(device);

        let height_atlas = terrain_data.create_node_atlas(device);

        let config = terrain_data.config;

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

        let indirect_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            contents: &[0; INDIRECT_BUFFER_SIZE as usize],
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

        let terrain_config_uniform: TerrainConfigUniform = (&config).into();
        let terrain_config_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: terrain_config_uniform.as_std140().as_bytes(),
        });

        let terrain_data_bind_group = device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: terrain_config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&height_atlas.texture_view),
                },
            ],
            label: None,
            layout: &terrain_pipeline.terrain_data_layout,
        });

        Ok(GpuTerrainData {
            config,
            quadtree_data,
            indirect_buffer,
            height_atlas,
            node_buffer_bind_groups,
            node_parameter_bind_group,
            patch_buffer_bind_group,
            patch_list_bind_group,
            terrain_data_bind_group,
        })
    }
}
