use crate::node_atlas::NodeAtlas;
use crate::quadtree_update::{GpuQuadtreeUpdate, NodeUpdate};
use crate::terrain::TerrainConfig;
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
    pub(crate) quadtree_texture: Texture,
    pub(crate) quadtree_data: Vec<(BufferVec<NodeUpdate>, TextureView)>,
    pub(crate) node_temp_buffers: [Buffer; 2],
    pub(crate) node_final_buffer: Buffer,
    pub(crate) parameter_buffer: Buffer,
    pub(crate) indirect_buffer: Buffer,
    pub(crate) config_uniform: Buffer,
    pub(crate) node_buffer_bind_groups: [BindGroup; 2],
    pub(crate) node_parameter_bind_group: BindGroup,
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

    fn create_node_buffers(&mut self, device: &RenderDevice) -> ([Buffer; 2], Buffer) {
        let max_node_count = self.config.chunk_count.x * self.config.chunk_count.x;

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
}

const INDIRECT_BUFFER_SIZE: BufferAddress = 5 * mem::size_of::<u32>() as BufferAddress;
const PARAMETER_BUFFER_SIZE: BufferAddress = 4 * mem::size_of::<u32>() as BufferAddress; // minimum buffer size = 16
const CONFIG_BUFFER_SIZE: BufferAddress = 4 * mem::size_of::<u32>() as BufferAddress;

const UPDATE_QUADTREE_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
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
};
pub const NODE_BUFFER_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Uint,
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PARAMETER_BUFFER_SIZE),
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(mem::size_of::<u32>() as u64),
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(mem::size_of::<u32>() as u64),
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::all(),
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(mem::size_of::<u32>() as u64),
            },
            count: None,
        },
    ],
};
const NODE_PARAMETER_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(INDIRECT_BUFFER_SIZE),
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PARAMETER_BUFFER_SIZE),
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(CONFIG_BUFFER_SIZE),
            },
            count: None,
        },
    ],
};

impl RenderAsset for PreparationData {
    type ExtractedAsset = PreparationData;
    type PreparedAsset = GpuPreparationData;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<TerrainComputePipeline>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        mut preparation: Self::ExtractedAsset,
        (device, queue, pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let quadtree_texture = preparation.create_quadtree_texture(&device, &queue);

        let (node_temp_buffers, node_final_buffer) = preparation.create_node_buffers(device);

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
                layout: &pipeline.node_buffer_bind_group_layout,
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
                layout: &pipeline.node_buffer_bind_group_layout,
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
            layout: &pipeline.node_parameter_bind_group_layout,
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

        Ok(GpuPreparationData {
            lod_count: config.lod_count as usize,
            quadtree_texture,
            quadtree_data,
            node_temp_buffers,
            node_final_buffer,
            parameter_buffer,
            indirect_buffer,
            config_uniform,
            node_buffer_bind_groups,
            node_parameter_bind_group,
        })
    }
}

pub struct TerrainComputePipeline {
    pub(crate) update_quadtree_bind_group_layout: BindGroupLayout,
    pub(crate) node_buffer_bind_group_layout: BindGroupLayout,
    pub(crate) node_parameter_bind_group_layout: BindGroupLayout,
    pub(crate) update_quadtree_pipeline: ComputePipeline,
    pub(crate) build_area_list_pipeline: ComputePipeline,
    pub(crate) build_node_list_pipeline: ComputePipeline,
    pub(crate) build_chunk_list_pipeline: ComputePipeline,
    pub(crate) reset_node_list_pipeline: ComputePipeline,
    pub(crate) prepare_next_node_list_pipeline: ComputePipeline,
    pub(crate) prepare_render_node_list_pipeline: ComputePipeline,
}

impl TerrainComputePipeline {
    fn create_update_quadtree_pipeline(
        device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
    ) -> ComputePipeline {
        let shader_source = include_str!("../../../assets/shaders/update_quadtree.wgsl");
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "update_quadtree",
        })
    }

    fn create_build_node_list_pipelines(
        device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
    ) -> (ComputePipeline, ComputePipeline, ComputePipeline) {
        let shader_source = include_str!("../../../assets/shaders/build_node_list.wgsl");
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        (
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "build_area_list",
            }),
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "build_node_list",
            }),
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "build_chunk_list",
            }),
        )
    }

    fn create_update_node_list_pipelines(
        device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
    ) -> (ComputePipeline, ComputePipeline, ComputePipeline) {
        let shader_source = include_str!("../../../assets/shaders/update_node_list.wgsl");
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        (
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "reset",
            }),
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "prepare_next",
            }),
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "prepare_render",
            }),
        )
    }
}

impl FromWorld for TerrainComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.get_resource::<RenderDevice>().unwrap();

        let update_quadtree_bind_group_layout =
            device.create_bind_group_layout(&UPDATE_QUADTREE_LAYOUT);
        let node_buffer_bind_group_layout = device.create_bind_group_layout(&NODE_BUFFER_LAYOUT);
        let node_parameter_bind_group_layout =
            device.create_bind_group_layout(&NODE_PARAMETER_LAYOUT);

        let update_quadtree_pipeline = TerrainComputePipeline::create_update_quadtree_pipeline(
            device,
            &update_quadtree_bind_group_layout,
        );

        let (build_area_list_pipeline, build_node_list_pipeline, build_chunk_list_pipeline) =
            TerrainComputePipeline::create_build_node_list_pipelines(
                device,
                &node_buffer_bind_group_layout,
            );

        let (
            reset_node_list_pipeline,
            prepare_next_node_list_pipeline,
            prepare_render_node_list_pipeline,
        ) = TerrainComputePipeline::create_update_node_list_pipelines(
            device,
            &node_parameter_bind_group_layout,
        );

        TerrainComputePipeline {
            update_quadtree_bind_group_layout,
            node_buffer_bind_group_layout,
            node_parameter_bind_group_layout,
            update_quadtree_pipeline,
            build_area_list_pipeline,
            build_node_list_pipeline,
            build_chunk_list_pipeline,
            reset_node_list_pipeline,
            prepare_next_node_list_pipeline,
            prepare_render_node_list_pipeline,
        }
    }
}

pub struct TerrainComputeNode {
    query: QueryState<(Read<GpuQuadtreeUpdate>, Read<Handle<PreparationData>>)>,
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
        let preparation_data = world
            .get_resource::<RenderAssets<PreparationData>>()
            .unwrap();

        let mut pass = context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&pipeline.update_quadtree_pipeline);

        for (gpu_quadtree_update, handle) in self.query.iter_manual(world) {
            for (count, bind_group) in &gpu_quadtree_update.0 {
                // if *count == 0 {
                //     continue;
                // }

                pass.set_bind_group(0, bind_group, &[]);
                pass.dispatch(*count, 1, 1);
            }

            let gpu_preparation_data = preparation_data.get(handle).unwrap();

            pass.set_bind_group(0, &gpu_preparation_data.node_parameter_bind_group, &[]);
            pass.set_pipeline(&pipeline.reset_node_list_pipeline);
            pass.dispatch(1, 1, 1);

            pass.set_bind_group(0, &gpu_preparation_data.node_buffer_bind_groups[1], &[]);
            pass.set_pipeline(&pipeline.build_area_list_pipeline);
            pass.dispatch_indirect(&gpu_preparation_data.indirect_buffer, 0);

            pass.set_bind_group(0, &gpu_preparation_data.node_parameter_bind_group, &[]);
            pass.set_pipeline(&pipeline.prepare_next_node_list_pipeline);
            pass.dispatch(1, 1, 1);

            for i in 0..gpu_preparation_data.lod_count - 1 {
                let index = i % 2;

                pass.set_bind_group(0, &gpu_preparation_data.node_buffer_bind_groups[index], &[]);
                pass.set_pipeline(&pipeline.build_node_list_pipeline);
                pass.dispatch_indirect(&gpu_preparation_data.indirect_buffer, 0);

                pass.set_bind_group(0, &gpu_preparation_data.node_parameter_bind_group, &[]);
                pass.set_pipeline(&pipeline.prepare_next_node_list_pipeline);
                pass.dispatch(1, 1, 1);
            }

            let index = (gpu_preparation_data.lod_count - 1) % 2;

            // build chunk list
            pass.set_bind_group(0, &gpu_preparation_data.node_buffer_bind_groups[index], &[]);
            pass.set_pipeline(&pipeline.build_chunk_list_pipeline);
            pass.dispatch_indirect(&gpu_preparation_data.indirect_buffer, 0);

            pass.set_bind_group(0, &gpu_preparation_data.node_parameter_bind_group, &[]);
            pass.set_pipeline(&pipeline.prepare_render_node_list_pipeline);
            pass.dispatch(1, 1, 1);
        }

        Ok(())
    }
}

pub struct SetPreparationDataBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetPreparationDataBindGroup<I> {
    type Param = (
        SRes<RenderAssets<PreparationData>>,
        SQuery<Read<Handle<PreparationData>>>,
    );

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (preparation_data, preparation_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let handle = preparation_query.get(item).unwrap();

        let gpu_preparation_data = match preparation_data.into_inner().get(handle) {
            Some(gpu_preparation_data) => gpu_preparation_data,
            None => return RenderCommandResult::Failure,
        };

        pass.set_bind_group(I, &gpu_preparation_data.node_buffer_bind_groups[0], &[]);

        RenderCommandResult::Success
    }
}
