use crate::debug::{info, TerrainDebugInfo};
use crate::quadtree::{traverse_quadtree, update_load_status, update_nodes, ViewDistance};
use crate::render::{
    pipeline::{queue_terrain, DrawTerrain, TerrainPipeline},
    terrain_data::TerrainData,
};
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

pub mod bundles;
pub mod debug;
pub mod preprocess;
pub mod quadtree;
pub mod render;
pub mod terrain;
pub mod tile;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        register_inspectable(app);

        app.add_asset::<TerrainData>()
            .add_plugin(RenderAssetPlugin::<TerrainData>::default())
            .add_plugin(ExtractComponentPlugin::<Handle<TerrainData>>::default());

        app.sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain>()
            .init_resource::<TerrainPipeline>()
            .init_resource::<SpecializedPipelines<TerrainPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_terrain);

        app.add_system(traverse_quadtree.before("update_nodes"))
            .add_system(update_nodes.label("update_nodes"))
            .add_system(update_load_status)
            .add_system(info.after("update_nodes"));
    }
}

fn register_inspectable(app: &mut App) {
    app.register_inspectable::<ViewDistance>()
        .register_inspectable::<TerrainDebugInfo>();
}
