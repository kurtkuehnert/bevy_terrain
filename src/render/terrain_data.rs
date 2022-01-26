use crate::{quadtree::NodeAtlas, render::pipeline::TerrainPipeline, terrain::TerrainConfig};
use bevy::{
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            std140::{AsStd140, Std140},
            std430::{AsStd430, Std430},
            *,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use std::{num::NonZeroU32, ops::Deref};

#[derive(Clone, Copy, Debug, AsStd430)]
#[repr(C)]
pub(crate) struct PatchData {
    position: UVec2,
    size: u32,
    range: f32,
    color: Vec4,
}

#[derive(Clone, Default, AsStd140)]
struct TerrainUniformData {
    height: f32,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "32a1cd80-cef4-4534-b0ec-bc3a3d0800a9"]
pub struct TerrainData {
    // Todo: consider terrain resources rename
    pub config: TerrainConfig,
    pub height_texture: Handle<Image>, // Todo: replace in favor of the node atlas
}

pub struct GpuTerrainData {
    pub(crate) quadtree_texture: Texture,
    pub(crate) draw_indirect_buffer: Buffer,
    pub(crate) patch_buffer: Buffer,
    pub(crate) terrain_uniform_buffer: Buffer,
    pub(crate) bind_group: BindGroup,
}

impl TerrainData {
    pub(crate) fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // terrain uniform data
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            TerrainUniformData::std140_size_static() as u64
                        ),
                    },
                    count: None,
                },
                // patch data
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(PatchData::std430_size_static() as u64),
                    },
                    count: None,
                },
                // height texture
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // height texture sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("terrain_data_layout"),
        })
    }

    fn create_quadtree_texture(&mut self, device: &RenderDevice, queue: &RenderQueue) -> Texture {
        let config = &self.config;

        let texture_descriptor = TextureDescriptor {
            label: None,
            size: Extent3d {
                width: config.chunk_count.x,
                height: config.chunk_count.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: config.lod_count, // one mip level per lod
            sample_count: 1,
            dimension: TextureDimension::D2,
            // only r16 required, but storage textures only support r32 https://www.w3.org/TR/WGSL/#texel-formats
            format: TextureFormat::R32Uint,
            usage: TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING,
        };

        let quadtree_texture = device.create_texture(&texture_descriptor);

        // Todo: generate data all at once and only specify the offset
        // use https://docs.rs/wgpu/latest/wgpu/util/trait.DeviceExt.html#tymethod.create_buffer_init

        for lod in 0..config.lod_count {
            let node_count = config.nodes_count(lod);

            let texture = ImageCopyTextureBase {
                texture: quadtree_texture.deref(),
                mip_level: lod,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All, // Todo: ?
            };

            let data_layout = ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::try_from(node_count.x * 4).unwrap()),
                rows_per_image: Some(NonZeroU32::try_from(node_count.y).unwrap()),
            };

            let size = Extent3d {
                width: node_count.x,
                height: node_count.y,
                depth_or_array_layers: 1,
            };

            let data: Vec<u32> = (0..node_count.x * node_count.y)
                .map(|_| NodeAtlas::INACTIVE_ID as u32)
                .collect();

            queue.write_texture(texture, bytemuck::cast_slice(&data), data_layout, size);
        }

        quadtree_texture
    }
}

impl RenderAsset for TerrainData {
    type ExtractedAsset = TerrainData;
    type PreparedAsset = GpuTerrainData;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<TerrainPipeline>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        mut terrain: Self::ExtractedAsset,
        (device, queue, pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        println!("init gpu terrain");

        let quadtree_texture = terrain.create_quadtree_texture(&device, &queue);

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: 5 * 4,
            usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        };

        let draw_indirect_buffer = device.create_buffer(&buffer_descriptor);

        let data: [u32; 5] = [640, 4, 0, 0, 0];

        queue.write_buffer(&draw_indirect_buffer, 0, bytemuck::cast_slice(&data));

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: 10000, // Todo: calculate this properly
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        };

        let patch_buffer = device.create_buffer(&buffer_descriptor);

        let data = [
            PatchData {
                position: UVec2::new(0, 0),
                size: 100,
                range: 0.0,
                color: Color::RED.into(),
            },
            PatchData {
                position: UVec2::new(100, 0),
                size: 100,
                range: 0.0,
                color: Color::RED.into(),
            },
            PatchData {
                position: UVec2::new(0, 100),
                size: 100,
                range: 0.0,
                color: Color::RED.into(),
            },
            PatchData {
                position: UVec2::new(100, 100),
                size: 100,
                range: 0.0,
                color: Color::RED.into(),
            },
        ];

        queue.write_buffer(&patch_buffer, 0, data.as_std430().as_bytes());

        let (height_texture_view, height_texture_sampler) =
            match gpu_images.get(&terrain.height_texture) {
                Some(gpu_image) => (&gpu_image.texture_view, &gpu_image.sampler),
                None => return Err(PrepareAssetError::RetryNextUpdate(terrain)),
            };

        let terrain_uniform_data = TerrainUniformData { height: 100.0 };

        let terrain_uniform_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: terrain_uniform_data.as_std140().as_bytes(),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: terrain_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: patch_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(height_texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(height_texture_sampler),
                },
            ],
            label: Some("terrain_data_bind_group"),
            layout: &pipeline.terrain_data_layout,
        });

        Ok(GpuTerrainData {
            bind_group,
            terrain_uniform_buffer,
            quadtree_texture,
            draw_indirect_buffer,
            patch_buffer,
        })
    }
}

pub struct SetTerrainDataBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetTerrainDataBindGroup<I> {
    type Param = (
        SRes<RenderAssets<TerrainData>>,
        SQuery<Read<Handle<TerrainData>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (terrain_data, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let handle = terrain_query.get(item).unwrap();

        let gpu_terrain_data = match terrain_data.into_inner().get(handle) {
            Some(gpu_terrain_data) => gpu_terrain_data,
            None => return RenderCommandResult::Failure,
        };

        pass.set_bind_group(I, &gpu_terrain_data.bind_group, &[]);

        RenderCommandResult::Success
    }
}
