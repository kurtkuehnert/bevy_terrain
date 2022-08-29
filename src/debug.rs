//! Contains a debug resource and systems controlling it to visualize different internal
//! data of the plugin.
use crate::{TerrainViewComponents, TerrainViewConfig};
use bevy::{
    prelude::*,
    render::{Extract, RenderApp, RenderStage},
};

/// Adds a terrain debug config and debug control systems.
pub struct TerrainDebugPlugin;

impl Plugin for TerrainDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(toggle_debug)
            .add_system(change_config)
            .sub_app_mut(RenderApp)
            .init_resource::<DebugTerrain>()
            .add_system_to_stage(RenderStage::Extract, extract_debug);
    }
}

#[derive(Clone, Resource)]
pub struct DebugTerrain {
    pub wireframe: bool,

    pub show_tiles: bool,
    pub show_lod: bool,
    pub show_uv: bool,

    pub circular_lod: bool,
    pub mesh_morph: bool,
    pub adaptive: bool,
    pub vertex_normal: bool,

    pub albedo: bool,
    pub bright: bool,
    pub lighting: bool,

    pub test1: bool,
    pub test2: bool,
    pub test3: bool,
}

impl Default for DebugTerrain {
    fn default() -> Self {
        Self {
            wireframe: false,
            show_tiles: false,
            show_lod: false,
            show_uv: false,
            circular_lod: true,
            mesh_morph: true,
            adaptive: false,
            vertex_normal: false,
            albedo: false,
            bright: false,
            lighting: true,
            test1: false,
            test2: false,
            test3: false,
        }
    }
}

pub fn toggle_debug(input: Res<Input<KeyCode>>, mut debug: ResMut<DebugTerrain>) {
    if input.just_pressed(KeyCode::W) {
        debug.wireframe = !debug.wireframe;
    }

    if input.just_pressed(KeyCode::P) {
        debug.show_tiles = !debug.show_tiles;
    }
    if input.just_pressed(KeyCode::L) {
        debug.show_lod = !debug.show_lod;
    }
    if input.just_pressed(KeyCode::U) {
        debug.show_uv = !debug.show_uv;
    }

    if input.just_pressed(KeyCode::N) {
        debug.circular_lod = !debug.circular_lod;
    }
    if input.just_pressed(KeyCode::M) {
        debug.mesh_morph = !debug.mesh_morph;
    }
    if input.just_pressed(KeyCode::D) {
        debug.adaptive = !debug.adaptive;
    }

    if input.just_pressed(KeyCode::A) {
        debug.albedo = !debug.albedo;
    }
    if input.just_pressed(KeyCode::B) {
        debug.bright = !debug.bright;
    }
    if input.just_pressed(KeyCode::S) {
        debug.lighting = !debug.lighting;
    }
    if input.just_pressed(KeyCode::V) {
        debug.vertex_normal = !debug.vertex_normal;
    }

    if input.just_pressed(KeyCode::Key1) {
        debug.test1 = !debug.test1;
    }
    if input.just_pressed(KeyCode::Key2) {
        debug.test2 = !debug.test2;
    }
    if input.just_pressed(KeyCode::Key3) {
        debug.test3 = !debug.test3;
    }
}

pub fn extract_debug(mut debug: ResMut<DebugTerrain>, extracted_debug: Extract<Res<DebugTerrain>>) {
    *debug = extracted_debug.clone();
}

pub fn change_config(
    input: Res<Input<KeyCode>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    for config in view_configs.0.values_mut() {
        if input.just_pressed(KeyCode::X) && config.tile_scale > 0.25 {
            config.tile_scale *= 0.95;
        }
        if input.just_pressed(KeyCode::Q) {
            config.tile_scale *= 1.05;
        }

        if input.just_pressed(KeyCode::I) {
            config.view_distance *= 0.95;
        }
        if input.just_pressed(KeyCode::O) {
            config.view_distance *= 1.05;
        }
    }
}
