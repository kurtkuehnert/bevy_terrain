use crate::{
    formats::tiff::TiffLoader,
    preprocess::{
        gpu_preprocessor::{
            create_downsample_layout, create_split_layout, create_stitch_layout, GpuPreprocessor,
        },
        preprocessor::{preprocessor_load_tile, select_ready_tasks, PreprocessTaskType},
    },
    shaders::{load_preprocess_shaders, DOWNSAMPLE_SHADER, SPLIT_SHADER, STITCH_SHADER},
    terrain::{Terrain, TerrainComponents},
    terrain_data::gpu_node_atlas::{create_attachment_layout, GpuNodeAtlas},
};
use bevy::{
    prelude::*,
    render::{
        graph::CameraDriverLabel,
        render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        Render, RenderApp, RenderSet,
    },
};
use itertools::Itertools;

pub mod gpu_preprocessor;
pub mod preprocessor;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct TerrainPreprocessLabel;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct TerrainPreprocessPipelineKey: u32 {
        const NONE       = 1 << 0;
        const SPLIT      = 1 << 1;
        const STITCH     = 1 << 2;
        const DOWNSAMPLE = 1 << 3;
    }
}

pub(crate) struct TerrainPreprocessItem {
    split_pipeline: CachedComputePipelineId,
    stitch_pipeline: CachedComputePipelineId,
    downsample_pipeline: CachedComputePipelineId,
}

impl TerrainPreprocessItem {
    fn pipelines<'a>(
        &'a self,
        pipeline_cache: &'a PipelineCache,
    ) -> Option<(&ComputePipeline, &ComputePipeline, &ComputePipeline)> {
        Some((
            pipeline_cache.get_compute_pipeline(self.split_pipeline)?,
            pipeline_cache.get_compute_pipeline(self.stitch_pipeline)?,
            pipeline_cache.get_compute_pipeline(self.downsample_pipeline)?,
        ))
    }

    pub(crate) fn is_loaded(&self, pipeline_cache: &PipelineCache) -> bool {
        pipeline_cache
            .get_compute_pipeline(self.split_pipeline)
            .is_some()
            && pipeline_cache
                .get_compute_pipeline(self.stitch_pipeline)
                .is_some()
            && pipeline_cache
                .get_compute_pipeline(self.downsample_pipeline)
                .is_some()
    }
}

#[derive(Resource)]
pub struct TerrainPreprocessPipelines {
    attachment_layout: BindGroupLayout,
    split_layout: BindGroupLayout,
    stitch_layout: BindGroupLayout,
    downsample_layout: BindGroupLayout,
    split_shader: Handle<Shader>,
    stitch_shader: Handle<Shader>,
    downsample_shader: Handle<Shader>,
}

impl FromWorld for TerrainPreprocessPipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let attachment_layout = create_attachment_layout(device);
        let split_layout = create_split_layout(device);
        let stitch_layout = create_stitch_layout(device);
        let downsample_layout = create_downsample_layout(device);

        let split_shader = asset_server.load(SPLIT_SHADER);
        let stitch_shader = asset_server.load(STITCH_SHADER);
        let downsample_shader = asset_server.load(DOWNSAMPLE_SHADER);

        Self {
            attachment_layout,
            split_layout,
            stitch_layout,
            downsample_layout,
            split_shader,
            stitch_shader,
            downsample_shader,
        }
    }
}

impl SpecializedComputePipeline for TerrainPreprocessPipelines {
    type Key = TerrainPreprocessPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let mut layout = default();
        let mut shader = default();
        let mut entry_point = default();

        let shader_defs = vec![];

        if key.contains(TerrainPreprocessPipelineKey::SPLIT) {
            layout = vec![self.attachment_layout.clone(), self.split_layout.clone()];
            shader = self.split_shader.clone();
            entry_point = "split".into();
        }
        if key.contains(TerrainPreprocessPipelineKey::STITCH) {
            layout = vec![self.attachment_layout.clone(), self.stitch_layout.clone()];
            shader = self.stitch_shader.clone();
            entry_point = "stitch".into();
        }
        if key.contains(TerrainPreprocessPipelineKey::DOWNSAMPLE) {
            layout = vec![
                self.attachment_layout.clone(),
                self.downsample_layout.clone(),
            ];
            shader = self.downsample_shader.clone();
            entry_point = "downsample".into();
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

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let preprocess_items = world.resource::<TerrainComponents<TerrainPreprocessItem>>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let preprocess_data = world.resource::<TerrainComponents<GpuPreprocessor>>();
        let gpu_node_atlases = world.resource::<TerrainComponents<GpuNodeAtlas>>();

        let terrains = self.terrain_query.iter_manual(world).collect_vec();

        context.add_command_buffer_generation_task(move |device| {
            let mut command_encoder =
                device.create_command_encoder(&CommandEncoderDescriptor::default());

            for terrain in terrains {
                let item = preprocess_items.get(&terrain).unwrap();

                let Some((split_pipeline, stitch_pipeline, downsample_pipeline)) =
                    item.pipelines(pipeline_cache)
                else {
                    continue;
                };

                let preprocess_data = preprocess_data.get(&terrain).unwrap();
                let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();

                for attachment in &gpu_node_atlas.attachments {
                    attachment.copy_nodes_to_write_section(&mut command_encoder);
                }

                if !preprocess_data.processing_tasks.is_empty() {
                    let mut compute_pass =
                        command_encoder.begin_compute_pass(&ComputePassDescriptor::default());

                    for task in &preprocess_data.processing_tasks {
                        let attachment =
                            &gpu_node_atlas.attachments[task.task.node.attachment_index as usize];

                        let pipeline = match task.task.task_type {
                            PreprocessTaskType::Split { .. } => split_pipeline,
                            PreprocessTaskType::Stitch { .. } => stitch_pipeline,
                            PreprocessTaskType::Downsample { .. } => downsample_pipeline,
                            _ => continue,
                        };

                        compute_pass.set_pipeline(pipeline);
                        compute_pass.set_bind_group(0, &attachment.bind_group, &[]);
                        compute_pass.set_bind_group(1, task.bind_group.as_ref().unwrap(), &[]);
                        compute_pass.dispatch_workgroups(
                            attachment.buffer_info.workgroup_count.x,
                            attachment.buffer_info.workgroup_count.y,
                            attachment.buffer_info.workgroup_count.z,
                        );
                    }
                }

                for attachment in &gpu_node_atlas.attachments {
                    attachment.copy_nodes_from_write_section(&mut command_encoder);

                    attachment.download_nodes(&mut command_encoder);

                    // if !attachment.atlas_write_slots.is_empty() {
                    //     println!(
                    //         "Ran preprocessing pipeline with {} nodes.",
                    //         attachment.atlas_write_slots.len()
                    //     )
                    // }
                }
            }

            command_encoder.finish()
        });

        Ok(())
    }
}

pub(crate) fn queue_terrain_preprocess(
    pipeline_cache: Res<PipelineCache>,
    preprocess_pipelines: ResMut<TerrainPreprocessPipelines>,
    mut pipelines: ResMut<SpecializedComputePipelines<TerrainPreprocessPipelines>>,
    mut preprocess_items: ResMut<TerrainComponents<TerrainPreprocessItem>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    for terrain in terrain_query.iter() {
        let split_pipeline = pipelines.specialize(
            &pipeline_cache,
            &preprocess_pipelines,
            TerrainPreprocessPipelineKey::SPLIT,
        );
        let stitch_pipeline = pipelines.specialize(
            &pipeline_cache,
            &preprocess_pipelines,
            TerrainPreprocessPipelineKey::STITCH,
        );
        let downsample_pipeline = pipelines.specialize(
            &pipeline_cache,
            &preprocess_pipelines,
            TerrainPreprocessPipelineKey::DOWNSAMPLE,
        );

        preprocess_items.insert(
            terrain,
            TerrainPreprocessItem {
                split_pipeline,
                stitch_pipeline,
                downsample_pipeline,
            },
        );
    }
}

pub struct TerrainPreprocessPlugin;

impl Plugin for TerrainPreprocessPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<TiffLoader>()
            .add_systems(Update, (select_ready_tasks, preprocessor_load_tile));

        app.sub_app_mut(RenderApp)
            .init_resource::<TerrainComponents<GpuPreprocessor>>()
            .init_resource::<TerrainComponents<TerrainPreprocessItem>>()
            .add_systems(
                ExtractSchedule,
                (
                    GpuPreprocessor::initialize,
                    GpuPreprocessor::extract.after(GpuPreprocessor::initialize),
                ),
            )
            .add_systems(
                Render,
                (
                    queue_terrain_preprocess.in_set(RenderSet::Queue),
                    GpuPreprocessor::prepare
                        .in_set(RenderSet::PrepareAssets)
                        .before(GpuNodeAtlas::prepare),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        load_preprocess_shaders(app);

        let render_app = app
            .sub_app_mut(RenderApp)
            .init_resource::<SpecializedComputePipelines<TerrainPreprocessPipelines>>()
            .init_resource::<TerrainPreprocessPipelines>();

        let preprocess_node = TerrainPreprocessNode::from_world(render_app.world_mut());
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(TerrainPreprocessLabel, preprocess_node);
        render_graph.add_node_edge(TerrainPreprocessLabel, CameraDriverLabel);
    }
}
