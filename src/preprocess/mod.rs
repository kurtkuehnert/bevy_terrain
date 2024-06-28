use crate::{
    formats::tiff::TiffLoader,
    preprocess::{
        gpu_preprocessor::GpuPreprocessor,
        preprocess_pipeline::{
            queue_terrain_preprocess, TerrainPreprocessItem, TerrainPreprocessLabel,
            TerrainPreprocessNode, TerrainPreprocessPipelines,
        },
        preprocessor::{preprocessor_load_tile, select_ready_tasks},
        shaders::load_preprocess_shaders,
    },
    terrain::TerrainComponents,
    terrain_data::gpu_node_atlas::GpuNodeAtlas,
};
use bevy::{
    prelude::*,
    render::{
        graph::CameraDriverLabel, render_graph::RenderGraph,
        render_resource::SpecializedComputePipelines, Render, RenderApp, RenderSet,
    },
};

pub mod gpu_preprocessor;
pub mod preprocess_pipeline;
pub mod preprocessor;
pub mod shaders;

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
