use crate::{
    node_atlas::GpuNodeAtlas,
    render::{
        culling::CullingBindGroup,
        layouts::*,
        terrain_data::{GpuTerrainData, TerrainData},
    },
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
use wgpu::ComputePass;

pub struct TerrainComputePipelines {
    pub(crate) prepare_indirect_layout: BindGroupLayout,
    pub(crate) update_quadtree_layout: BindGroupLayout,
    pub(crate) build_node_list_layout: BindGroupLayout,
    pub(crate) build_patch_list_layout: BindGroupLayout,
    pub(crate) build_chunk_maps_layout: BindGroupLayout,
    pub(crate) cull_data_layout: BindGroupLayout,
    prepare_area_list_pipeline: ComputePipeline,
    prepare_node_list_pipeline: ComputePipeline,
    prepare_patch_list_pipeline: ComputePipeline,
    prepare_render_pipeline: ComputePipeline,
    update_quadtree_pipeline: ComputePipeline,
    build_area_list_pipeline: ComputePipeline,
    build_node_list_pipeline: ComputePipeline,
    build_chunk_list_pipeline: ComputePipeline,
    build_patch_list_pipeline: ComputePipeline,
    build_chunk_maps_pipeline: ComputePipeline,
}

impl TerrainComputePipelines {
    fn create_prepare_indirect_pipelines(
        device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
    ) -> (
        ComputePipeline,
        ComputePipeline,
        ComputePipeline,
        ComputePipeline,
    ) {
        let shader_source = include_str!("shaders/prepare_indirect.wgsl");
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
                entry_point: "prepare_area_list",
            }),
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "prepare_node_list",
            }),
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "prepare_patch_list",
            }),
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "prepare_render",
            }),
        )
    }

    fn create_update_quadtree_pipeline(
        device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
    ) -> ComputePipeline {
        let shader_source = include_str!("shaders/update_quadtree.wgsl");
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
        let shader_source = include_str!("shaders/build_node_list.wgsl");
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

    fn create_build_patch_list_pipeline(
        device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
        cull_data_layout: &BindGroupLayout,
    ) -> ComputePipeline {
        let shader_source = include_str!("shaders/build_patch_list.wgsl");
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout, cull_data_layout],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "build_patch_list",
        })
    }

    fn create_build_chunk_maps_pipeline(
        device: &RenderDevice,
        bind_group_layout: &BindGroupLayout,
    ) -> ComputePipeline {
        let shader_source = include_str!("shaders/build_chunk_maps.wgsl");
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
            entry_point: "build_chunk_maps",
        })
    }
}

impl FromWorld for TerrainComputePipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let update_quadtree_layout = device.create_bind_group_layout(&UPDATE_QUADTREE_LAYOUT);
        let build_node_list_layout = device.create_bind_group_layout(&BUILD_NODE_LIST_LAYOUT);
        let prepare_indirect_layout = device.create_bind_group_layout(&PREPARE_INDIRECT_LAYOUT);
        let build_patch_list_layout = device.create_bind_group_layout(&BUILD_PATCH_LIST_LAYOUT);
        let build_chunk_maps_layout = device.create_bind_group_layout(&BUILD_CHUNK_MAPS_LAYOUT);
        let cull_data_layout = device.create_bind_group_layout(&CULL_DATA_LAYOUT);
        let (
            prepare_area_list_pipeline,
            prepare_node_list_pipeline,
            prepare_patch_list_pipeline,
            prepare_render_pipeline,
        ) = TerrainComputePipelines::create_prepare_indirect_pipelines(
            device,
            &prepare_indirect_layout,
        );

        let update_quadtree_pipeline = TerrainComputePipelines::create_update_quadtree_pipeline(
            device,
            &update_quadtree_layout,
        );

        let (build_area_list_pipeline, build_node_list_pipeline, build_chunk_list_pipeline) =
            TerrainComputePipelines::create_build_node_list_pipelines(
                device,
                &build_node_list_layout,
            );

        let build_patch_list_pipeline = TerrainComputePipelines::create_build_patch_list_pipeline(
            device,
            &build_patch_list_layout,
            &cull_data_layout,
        );

        let build_chunk_maps_pipeline = TerrainComputePipelines::create_build_chunk_maps_pipeline(
            device,
            &build_chunk_maps_layout,
        );

        TerrainComputePipelines {
            prepare_indirect_layout,
            update_quadtree_layout,
            build_node_list_layout,
            build_patch_list_layout,
            build_chunk_maps_layout,
            cull_data_layout,
            prepare_area_list_pipeline,
            prepare_node_list_pipeline,
            prepare_patch_list_pipeline,
            prepare_render_pipeline,
            update_quadtree_pipeline,
            build_area_list_pipeline,
            build_node_list_pipeline,
            build_chunk_list_pipeline,
            build_patch_list_pipeline,
            build_chunk_maps_pipeline,
        }
    }
}

pub struct TerrainComputeNode {
    query: QueryState<(
        Option<Read<GpuNodeAtlas>>,
        Read<Handle<TerrainData>>,
        Read<CullingBindGroup>,
    )>,
}

impl FromWorld for TerrainComputeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query(),
        }
    }
}

impl TerrainComputeNode {
    fn update_quadtree<'a>(
        pass: &mut ComputePass<'a>,
        gpu_node_atlas: Option<&GpuNodeAtlas>,
        gpu_terrain_data: &'a GpuTerrainData,
    ) {
        if let Some(gpu_node_atlas) = gpu_node_atlas {
            for (count, bind_group) in gpu_node_atlas
                .node_update_counts
                .iter()
                .zip(&gpu_terrain_data.update_quadtree_bind_groups)
            {
                pass.set_bind_group(0, bind_group, &[]);
                pass.dispatch(*count, 1, 1);
            }
        }
    }

    fn build_node_list<'a>(
        pass: &mut ComputePass<'a>,
        compute_pipelines: &'a TerrainComputePipelines,
        gpu_terrain_data: &'a GpuTerrainData,
    ) {
        pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
        pass.set_pipeline(&compute_pipelines.prepare_area_list_pipeline);
        pass.dispatch(1, 1, 1);

        pass.set_bind_group(0, &gpu_terrain_data.build_node_list_bind_groups[1], &[]);
        pass.set_pipeline(&compute_pipelines.build_area_list_pipeline);
        pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

        pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
        pass.set_pipeline(&compute_pipelines.prepare_node_list_pipeline);
        pass.dispatch(1, 1, 1);

        let count = gpu_terrain_data.config.lod_count as usize - 1;

        for i in 0..count {
            let index = i % 2;

            pass.set_bind_group(0, &gpu_terrain_data.build_node_list_bind_groups[index], &[]);
            pass.set_pipeline(&compute_pipelines.build_node_list_pipeline);
            pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

            pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
            pass.set_pipeline(&compute_pipelines.prepare_node_list_pipeline);
            pass.dispatch(1, 1, 1);
        }

        // build chunk list
        pass.set_bind_group(
            0,
            &gpu_terrain_data.build_node_list_bind_groups[count % 2],
            &[],
        );
        pass.set_pipeline(&compute_pipelines.build_chunk_list_pipeline);
        pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);
    }

    fn build_patch_list<'a>(
        pass: &mut ComputePass<'a>,
        compute_pipelines: &'a TerrainComputePipelines,
        gpu_terrain_data: &'a GpuTerrainData,
    ) {
        pass.set_bind_group(0, &gpu_terrain_data.build_patch_list_bind_group, &[]);
        pass.set_pipeline(&compute_pipelines.build_patch_list_pipeline);
        pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

        pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
        pass.set_pipeline(&compute_pipelines.prepare_render_pipeline);
        pass.dispatch(1, 1, 1);
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
        let compute_pipelines = world.resource::<TerrainComputePipelines>();
        let terrain_data = world.resource::<RenderAssets<TerrainData>>();

        let mut pass = context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(&compute_pipelines.update_quadtree_pipeline);

        for (gpu_node_atlas, handle, culling_bind_group) in self.query.iter_manual(world) {
            let gpu_terrain_data = terrain_data.get(handle).unwrap();

            TerrainComputeNode::update_quadtree(&mut pass, gpu_node_atlas, gpu_terrain_data);
            TerrainComputeNode::build_node_list(&mut pass, compute_pipelines, gpu_terrain_data);

            pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
            pass.set_pipeline(&compute_pipelines.prepare_patch_list_pipeline);
            pass.dispatch(1, 1, 1);

            pass.set_bind_group(0, &gpu_terrain_data.build_chunk_maps_bind_group, &[]);
            pass.set_pipeline(&compute_pipelines.build_chunk_maps_pipeline);
            pass.dispatch(
                gpu_terrain_data.config.chunk_count.x * gpu_terrain_data.config.chunk_count.y,
                1,
                1,
            );

            pass.set_bind_group(1, &culling_bind_group.value, &[]);
            TerrainComputeNode::build_patch_list(&mut pass, compute_pipelines, gpu_terrain_data);
        }

        Ok(())
    }
}
