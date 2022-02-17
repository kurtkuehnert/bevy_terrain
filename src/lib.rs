use crate::render::terrain_pipeline::extract_terrain;
use crate::{
    debug::{info, TerrainDebugInfo},
    node_atlas::{queue_atlas_updates, queue_quadtree_update, NodeAtlas},
    quadtree::{traverse_quadtree, update_load_status, update_nodes, ViewDistance},
    render::{
        terrain_data::TerrainData,
        terrain_pipeline::{queue_terrain, DrawTerrain, TerrainPipeline},
    },
};
use bevy::{
    core_pipeline::{node::MAIN_PASS_DEPENDENCIES, Opaque3d},
    prelude::*,
    render::{
        render_asset::RenderAssetPlugin, render_component::ExtractComponentPlugin,
        render_graph::RenderGraph, render_phase::AddRenderCommand,
        render_resource::SpecializedPipelines, RenderApp, RenderStage,
    },
};
use bevy_inspector_egui::RegisterInspectable;
use render::compute_pipeline::{TerrainComputeNode, TerrainComputePipeline};

pub mod bundles;
pub mod config;
pub mod debug;
pub mod node_atlas;
pub mod preprocess;
pub mod quadtree;
pub mod render;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        register_inspectable(app);

        app.add_asset::<TerrainData>()
            .add_plugin(RenderAssetPlugin::<TerrainData>::default())
            .add_plugin(ExtractComponentPlugin::<Handle<TerrainData>>::default())
            .add_plugin(ExtractComponentPlugin::<NodeAtlas>::default());

        app.add_system(traverse_quadtree.before("update_nodes"))
            .add_system(update_nodes.label("update_nodes"))
            .add_system(update_load_status)
            .add_system(info.after("update_nodes"));

        let render_app = app
            .sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain>()
            .init_resource::<TerrainComputePipeline>()
            .init_resource::<TerrainPipeline>()
            .init_resource::<SpecializedPipelines<TerrainPipeline>>()
            .add_system_to_stage(RenderStage::Extract, extract_terrain)
            .add_system_to_stage(RenderStage::Queue, queue_quadtree_update)
            .add_system_to_stage(RenderStage::Queue, queue_atlas_updates)
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
