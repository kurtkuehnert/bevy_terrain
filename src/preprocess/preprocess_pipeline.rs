use crate::{
    preprocess::{
        gpu_preprocessor::{
            create_downsample_layout, create_split_layout, create_stitch_layout, GpuPreprocessor,
        },
        preprocessor::PreprocessTaskType,
        shaders::{DOWNSAMPLE_SHADER, SPLIT_SHADER, STITCH_SHADER},
    },
    terrain::{Terrain, TerrainComponents},
    terrain_data::gpu_node_atlas::{create_attachment_layout, GpuNodeAtlas},
};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct TerrainPreprocessLabel;

type TerrainPreprocessPipelineKey = TerrainPreprocessPipelineId;

#[derive(Copy, Clone, Hash, PartialEq, Eq, EnumIter)]
pub enum TerrainPreprocessPipelineId {
    Split,
    Stitch,
    Downsample,
}

#[derive(Resource)]
pub struct TerrainPreprocessPipelines {
    attachment_layout: BindGroupLayout,
    split_layout: BindGroupLayout,
    stitch_layout: BindGroupLayout,
    downsample_layout: BindGroupLayout,
    pipelines: Vec<CachedComputePipelineId>,
}

impl FromWorld for TerrainPreprocessPipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let attachment_layout = create_attachment_layout(device);
        let split_layout = create_split_layout(device);
        let stitch_layout = create_stitch_layout(device);
        let downsample_layout = create_downsample_layout(device);

        let mut preprocess_pipelines = TerrainPreprocessPipelines {
            attachment_layout,
            split_layout,
            stitch_layout,
            downsample_layout,
            pipelines: vec![],
        };

        world.resource_scope(|world: &mut World,mut pipelines: Mut<SpecializedComputePipelines<TerrainPreprocessPipelines>>| {
            let pipeline_cache = world.resource::<PipelineCache>();
            for id in TerrainPreprocessPipelineId::iter() {
                preprocess_pipelines.pipelines.push(pipelines.specialize(pipeline_cache, &preprocess_pipelines, id));
            }
        });

        preprocess_pipelines
    }
}

impl SpecializedComputePipeline for TerrainPreprocessPipelines {
    type Key = TerrainPreprocessPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let layout;
        let shader;
        let entry_point;

        let shader_defs = vec![];

        match key {
            TerrainPreprocessPipelineId::Split => {
                layout = vec![self.attachment_layout.clone(), self.split_layout.clone()];
                shader = SPLIT_SHADER;
                entry_point = "split".into();
            }
            TerrainPreprocessPipelineId::Stitch => {
                layout = vec![self.attachment_layout.clone(), self.stitch_layout.clone()];
                shader = STITCH_SHADER;
                entry_point = "stitch".into();
            }
            TerrainPreprocessPipelineId::Downsample => {
                layout = vec![
                    self.attachment_layout.clone(),
                    self.downsample_layout.clone(),
                ];
                shader = DOWNSAMPLE_SHADER;
                entry_point = "downsample".into();
            }
        }

        ComputePipelineDescriptor {
            label: Some("terrain_preprocess_pipeline".into()),
            layout,
            push_constant_ranges: default(),
            shader,
            shader_defs,
            entry_point,
        }
    }
}

pub struct TerrainPreprocessNode {
    terrain_query: QueryState<Entity, With<Terrain>>,
}

impl FromWorld for TerrainPreprocessNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            terrain_query: world.query_filtered(),
        }
    }
}

impl render_graph::Node for TerrainPreprocessNode {
    fn update(&mut self, world: &mut World) {
        self.terrain_query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let preprocess_pipelines = world.resource::<TerrainPreprocessPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let preprocess_data = world.resource::<TerrainComponents<GpuPreprocessor>>();
        let gpu_node_atlases = world.resource::<TerrainComponents<GpuNodeAtlas>>();

        let pipelines = &match TerrainPreprocessPipelineId::iter()
            .map(|id| {
                pipeline_cache.get_compute_pipeline(preprocess_pipelines.pipelines[id as usize])
            })
            .collect::<Option<Vec<_>>>()
        {
            None => return Ok(()), // some pipelines are not loaded yet
            Some(pipelines) => pipelines,
        };

        for terrain in self.terrain_query.iter_manual(world) {
            let preprocess_data = preprocess_data.get(&terrain).unwrap();
            let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();

            for attachment in &gpu_node_atlas.attachments {
                attachment.copy_nodes_to_write_section(context.command_encoder());
            }

            if !preprocess_data.processing_tasks.is_empty() {
                let mut pass = context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor::default());

                for task in &preprocess_data.processing_tasks {
                    let attachment =
                        &gpu_node_atlas.attachments[task.task.node.attachment_index as usize];

                    pass.set_bind_group(0, &attachment.bind_group, &[]);

                    match task.task.task_type {
                        PreprocessTaskType::Split { .. } => {
                            // dbg!("running split shader");

                            pass.set_pipeline(
                                pipelines[TerrainPreprocessPipelineId::Split as usize],
                            );
                        }
                        PreprocessTaskType::Stitch { .. } => {
                            // dbg!("running stitch shader");

                            pass.set_pipeline(
                                pipelines[TerrainPreprocessPipelineId::Stitch as usize],
                            );
                        }
                        PreprocessTaskType::Downsample { .. } => {
                            // dbg!("running downsample shader");

                            pass.set_pipeline(
                                pipelines[TerrainPreprocessPipelineId::Downsample as usize],
                            );
                        }
                        _ => continue,
                    }

                    pass.set_bind_group(1, task.bind_group.as_ref().unwrap(), &[]);
                    pass.dispatch_workgroups(
                        attachment.buffer_info.workgroup_count.x,
                        attachment.buffer_info.workgroup_count.y,
                        attachment.buffer_info.workgroup_count.z,
                    );
                }
            }

            for attachment in &gpu_node_atlas.attachments {
                attachment.copy_nodes_from_write_section(context.command_encoder());

                attachment.download_nodes(context.command_encoder());

                // if !attachment.atlas_write_slots.is_empty() {
                //     println!(
                //         "Ran preprocessing pipeline with {} nodes.",
                //         attachment.atlas_write_slots.len()
                //     )
                // }
            }
        }

        Ok(())
    }
}
