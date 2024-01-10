use crate::{
    formats::tiff::TiffLoader,
    preprocess::{
        gpu_preprocessor::GpuPreprocessor,
        preprocess_pipeline::{TerrainPreprocessNode, TerrainPreprocessPipelines},
        preprocessor::{preprocessor_load_tile, select_ready_tasks},
        shaders::load_preprocess_shaders,
    },
    terrain::TerrainComponents,
    terrain_data::gpu_node_atlas::GpuNodeAtlas,
};
use bevy::{
    prelude::*,
    render::{
        main_graph::node::CAMERA_DRIVER, render_graph::RenderGraph,
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

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<TerrainComponents<GpuPreprocessor>>()
                .add_systems(
                    ExtractSchedule,
                    (
                        GpuPreprocessor::initialize,
                        GpuPreprocessor::extract.after(GpuPreprocessor::initialize),
                    ),
                )
                .add_systems(
                    Render,
                    GpuPreprocessor::prepare
                        .in_set(RenderSet::PrepareAssets)
                        .before(GpuNodeAtlas::prepare),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        load_preprocess_shaders(app);

        let render_app = app
            .sub_app_mut(RenderApp)
            .init_resource::<SpecializedComputePipelines<TerrainPreprocessPipelines>>()
            .init_resource::<TerrainPreprocessPipelines>();

        let preprocess_node = TerrainPreprocessNode::from_world(&mut render_app.world);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("terrain_preprocess", preprocess_node);
        render_graph.add_node_edge("terrain_preprocess", CAMERA_DRIVER);
    }
}
