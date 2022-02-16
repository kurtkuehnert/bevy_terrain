use crate::node_atlas::{GpuQuadtreeUpdate, NodeUpdate};
use crate::render::terrain_data::{
    TerrainData, CONFIG_BUFFER_SIZE, INDIRECT_BUFFER_SIZE, PARAMETER_BUFFER_SIZE, PATCH_SIZE,
};
use bevy::{
    ecs::system::lifetimeless::Read,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};
use std::mem;

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
pub const PATCH_BUFFER_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
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
                min_binding_size: BufferSize::new(mem::size_of::<u32>() as u64),
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::all(),
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PATCH_SIZE),
            },
            count: None,
        },
    ],
};

pub struct TerrainComputePipeline {
    pub(crate) update_quadtree_bind_group_layout: BindGroupLayout,
    pub(crate) node_buffer_bind_group_layout: BindGroupLayout,
    pub(crate) node_parameter_bind_group_layout: BindGroupLayout,
    pub(crate) patch_buffer_bind_group_layout: BindGroupLayout,
    pub(crate) update_quadtree_pipeline: ComputePipeline,
    pub(crate) build_area_list_pipeline: ComputePipeline,
    pub(crate) build_node_list_pipeline: ComputePipeline,
    pub(crate) build_chunk_list_pipeline: ComputePipeline,
    pub(crate) reset_node_list_pipeline: ComputePipeline,
    pub(crate) prepare_next_node_list_pipeline: ComputePipeline,
    pub(crate) prepare_render_node_list_pipeline: ComputePipeline,
    pub(crate) build_patch_list_pipeline: ComputePipeline,
    pub(crate) prepare_patch_list_pipeline: ComputePipeline,
}

impl TerrainComputePipeline {
    fn create_update_quadtree_pipeline(
        device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
    ) -> ComputePipeline {
        let shader_source = include_str!("../../../../assets/shaders/update_quadtree.wgsl");
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
        let shader_source = include_str!("../../../../assets/shaders/build_node_list.wgsl");
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
    ) -> (
        ComputePipeline,
        ComputePipeline,
        ComputePipeline,
        ComputePipeline,
    ) {
        let shader_source = include_str!("../../../../assets/shaders/update_node_list.wgsl");
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
                entry_point: "prepare_patch",
            }),
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "prepare_render",
            }),
        )
    }

    fn create_build_patch_list_pipeline(
        device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
    ) -> ComputePipeline {
        let shader_source = include_str!("../../../../assets/shaders/build_patch_list.wgsl");
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
            entry_point: "build_patch_list",
        })
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
        let patch_buffer_bind_group_layout = device.create_bind_group_layout(&PATCH_BUFFER_LAYOUT);

        let update_quadtree_pipeline = TerrainComputePipeline::create_update_quadtree_pipeline(
            device,
            &update_quadtree_bind_group_layout,
        );

        let (build_area_list_pipeline, build_node_list_pipeline, build_chunk_list_pipeline) =
            TerrainComputePipeline::create_build_node_list_pipelines(
                device,
                &node_buffer_bind_group_layout,
            );

        let build_patch_list_pipeline = TerrainComputePipeline::create_build_patch_list_pipeline(
            device,
            &patch_buffer_bind_group_layout,
        );

        let (
            reset_node_list_pipeline,
            prepare_next_node_list_pipeline,
            prepare_patch_list_pipeline,
            prepare_render_node_list_pipeline,
        ) = TerrainComputePipeline::create_update_node_list_pipelines(
            device,
            &node_parameter_bind_group_layout,
        );

        TerrainComputePipeline {
            update_quadtree_bind_group_layout,
            node_buffer_bind_group_layout,
            node_parameter_bind_group_layout,
            patch_buffer_bind_group_layout,
            update_quadtree_pipeline,
            build_area_list_pipeline,
            build_node_list_pipeline,
            build_chunk_list_pipeline,
            reset_node_list_pipeline,
            prepare_next_node_list_pipeline,
            prepare_render_node_list_pipeline,
            prepare_patch_list_pipeline,
            build_patch_list_pipeline,
        }
    }
}

pub struct TerrainComputeNode {
    query: QueryState<(Read<GpuQuadtreeUpdate>, Read<Handle<TerrainData>>)>,
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
        let terrain_data = world.get_resource::<RenderAssets<TerrainData>>().unwrap();

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

            let gpu_terrain_data = terrain_data.get(handle).unwrap();

            pass.set_bind_group(0, &gpu_terrain_data.node_parameter_bind_group, &[]);
            pass.set_pipeline(&pipeline.reset_node_list_pipeline);
            pass.dispatch(1, 1, 1);

            pass.set_bind_group(0, &gpu_terrain_data.node_buffer_bind_groups[1], &[]);
            pass.set_pipeline(&pipeline.build_area_list_pipeline);
            pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

            pass.set_bind_group(0, &gpu_terrain_data.node_parameter_bind_group, &[]);
            pass.set_pipeline(&pipeline.prepare_next_node_list_pipeline);
            pass.dispatch(1, 1, 1);

            for i in 0..gpu_terrain_data.config.lod_count as usize - 1 {
                let index = i % 2;

                pass.set_bind_group(0, &gpu_terrain_data.node_buffer_bind_groups[index], &[]);
                pass.set_pipeline(&pipeline.build_node_list_pipeline);
                pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

                pass.set_bind_group(0, &gpu_terrain_data.node_parameter_bind_group, &[]);
                pass.set_pipeline(&pipeline.prepare_next_node_list_pipeline);
                pass.dispatch(1, 1, 1);
            }

            let index = (gpu_terrain_data.config.lod_count as usize - 1) % 2;

            // build chunk list
            pass.set_bind_group(0, &gpu_terrain_data.node_buffer_bind_groups[index], &[]);
            pass.set_pipeline(&pipeline.build_chunk_list_pipeline);
            pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

            pass.set_bind_group(0, &gpu_terrain_data.node_parameter_bind_group, &[]);
            pass.set_pipeline(&pipeline.prepare_patch_list_pipeline);
            pass.dispatch(1, 1, 1);

            pass.set_bind_group(0, &gpu_terrain_data.patch_buffer_bind_group, &[]);
            pass.set_pipeline(&pipeline.build_patch_list_pipeline);
            pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

            pass.set_bind_group(0, &gpu_terrain_data.node_parameter_bind_group, &[]);
            pass.set_pipeline(&pipeline.prepare_render_node_list_pipeline);
            pass.dispatch(1, 1, 1);
        }

        Ok(())
    }
}
