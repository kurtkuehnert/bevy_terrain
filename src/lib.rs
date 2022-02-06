use crate::compute::{PreparationData, TerrainComputeNode, TerrainComputePipeline};
use crate::debug::{info, TerrainDebugInfo};
use crate::quadtree::ViewDistance;
use crate::quadtree_update::queue_quadtree_update;
use crate::render::{
    pipeline::{queue_terrain, DrawTerrain, TerrainPipeline},
    render_data::RenderData,
};
use bevy::core_pipeline::node::MAIN_PASS_DEPENDENCIES;
use bevy::render::render_graph::RenderGraph;
use bevy::{
    core_pipeline::Opaque3d,
    prelude::*,
    render::{
        render_asset::RenderAssetPlugin, render_component::ExtractComponentPlugin,
        render_phase::AddRenderCommand, render_resource::SpecializedPipelines, RenderApp,
        RenderStage,
    },
};
use bevy_inspector_egui::RegisterInspectable;
use quadtree_update::QuadtreeUpdate;
use systems::{traverse_quadtree, update_load_status, update_nodes};

pub mod bundles;
pub mod compute;
pub mod debug;
pub mod node_atlas;
pub mod preprocess;
pub mod quadtree;
pub mod quadtree_update;
pub mod render;
pub mod systems;
pub mod terrain;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        register_inspectable(app);

        app.add_asset::<RenderData>()
            .add_asset::<PreparationData>()
            .add_plugin(RenderAssetPlugin::<RenderData>::default())
            .add_plugin(RenderAssetPlugin::<PreparationData>::default())
            .add_plugin(ExtractComponentPlugin::<Handle<RenderData>>::default())
            .add_plugin(ExtractComponentPlugin::<Handle<PreparationData>>::default())
            .add_plugin(ExtractComponentPlugin::<QuadtreeUpdate>::default());

        app.add_system(traverse_quadtree.before("update_nodes"))
            .add_system(update_nodes.label("update_nodes"))
            .add_system(update_load_status)
            .add_system(info.after("update_nodes"));

        let render_app = app
            .sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain>()
            .init_resource::<TerrainComputePipeline>()
            .add_system_to_stage(RenderStage::Queue, queue_quadtree_update)
            .init_resource::<TerrainPipeline>()
            .init_resource::<SpecializedPipelines<TerrainPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_terrain);

        let compute_node = TerrainComputeNode::from_world(&mut render_app.world);

        let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_node("terrain_compute", compute_node);
        render_graph
            .add_node_edge("terrain_compute", MAIN_PASS_DEPENDENCIES)
            .unwrap();
    }
}

fn register_inspectable(app: &mut App) {
    app.register_inspectable::<ViewDistance>()
        .register_inspectable::<TerrainDebugInfo>();
}
