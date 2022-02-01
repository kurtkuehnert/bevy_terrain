use crate::node_atlas::NodeAtlas;
use crate::quadtree_update::{GpuQuadtreeUpdate, NodeUpdate};
use crate::terrain::TerrainConfig;
use bevy::ecs::system::lifetimeless::{Read, SRes};
use bevy::ecs::system::SystemParamItem;
use bevy::reflect::TypeUuid;
use bevy::render::render_asset::{PrepareAssetError, RenderAsset};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self},
        render_resource::*,
        renderer::RenderContext,
    },
};
use std::mem;
use std::num::NonZeroU32;
use std::ops::Deref;

pub struct GpuPreparationData {
    pub(crate) quadtree_texture: Texture,
    pub(crate) quadtree_data: Vec<(BufferVec<NodeUpdate>, TextureView)>,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "c0172535-3220-4d02-ae1f-c35dcfde98c3"]
pub struct PreparationData {
    // Todo: consider terrain resources rename
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
            usage: TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING,
        };

        let quadtree_texture = device.create_texture(&texture_descriptor);

        // Todo: use https://docs.rs/wgpu/latest/wgpu/util/trait.DeviceExt.html#tymethod.create_buffer_init once its added to bevy

        for lod in 0..config.lod_count {
            let node_count = config.nodes_count(lod);

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
}

impl RenderAsset for PreparationData {
    type ExtractedAsset = PreparationData;
    type PreparedAsset = GpuPreparationData;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        mut preparation: Self::ExtractedAsset,
        (device, queue): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let quadtree_texture = preparation.create_quadtree_texture(&device, &queue);

        let quadtree_data = (0..preparation.config.lod_count)
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

        Ok(GpuPreparationData {
            quadtree_texture,
            quadtree_data,
        })
    }
}

pub struct TerrainComputePipeline {
    pub(crate) quadtree_update_pipeline: ComputePipeline,
    pub(crate) quadtree_update_bind_group_layout: BindGroupLayout,
}

impl FromWorld for TerrainComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.get_resource::<RenderDevice>().unwrap();

        let shader_source = include_str!("../../../assets/shaders/update_quadtree.wgsl");
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let quadtree_update_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R16Uint,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(mem::size_of::<NodeUpdate>() as u64),
                        },
                        count: None,
                    },
                ],
            });

        let quadtree_update_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&quadtree_update_bind_group_layout],
                push_constant_ranges: &[],
            });

        let quadtree_update_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&quadtree_update_pipeline_layout),
            module: &shader,
            entry_point: "update_quadtree",
        });

        TerrainComputePipeline {
            quadtree_update_pipeline,
            quadtree_update_bind_group_layout,
        }
    }
}

pub struct TerrainComputeNode {
    query: QueryState<Read<GpuQuadtreeUpdate>>,
}

impl FromWorld for TerrainComputeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query(),
        }
    }
}

impl render_graph::Node for TerrainComputeNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline = world.get_resource::<TerrainComputePipeline>().unwrap();

        let mut pass = context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&pipeline.quadtree_update_pipeline);

        for gpu_quadtree_update in self.query.iter_manual(world) {
            for (count, bind_group) in &gpu_quadtree_update.0 {
                if count == 0 {
                    continue;
                }

                pass.set_bind_group(0, bind_group, &[]);
                pass.dispatch(*count, 1, 1);
            }
        }

        Ok(())
    }
}
