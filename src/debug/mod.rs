//! Contains a debug resource and systems controlling it to visualize different internal
//! data of the plugin.
use crate::{debug::camera::debug_camera_control, TerrainViewComponents, TerrainViewConfig};
use bevy::{
    prelude::*,
    render::{Extract, RenderApp},
};

pub mod camera;

/// Adds a terrain debug config, a debug camera and debug control systems.
pub struct TerrainDebugPlugin;

impl Plugin for TerrainDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugTerrain>()
            .add_system(debug_camera_control)
            .add_system(toggle_debug)
            .add_system(change_config);

        app.sub_app_mut(RenderApp)
            .init_resource::<DebugTerrain>()
            .add_system(extract_debug.in_schedule(ExtractSchedule));
    }
}

#[derive(Clone, Resource)]
pub struct DebugTerrain {
    pub wireframe: bool,
    pub show_tiles: bool,
    pub show_lod: bool,
    pub show_uv: bool,
    pub show_nodes: bool,
    pub show_minmax_error: bool,
    pub minmax: bool,
    pub mesh_morph: bool,
    pub albedo: bool,
    pub bright: bool,
    pub lighting: bool,
    pub sample_grad: bool,
    pub freeze: bool,
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
            show_nodes: false,
            show_minmax_error: false,
            minmax: false,
            mesh_morph: true,
            albedo: false,
            bright: false,
            lighting: true,
            sample_grad: true,
            freeze: false,
            test1: false,
            test2: false,
            test3: true,
        }
    }
}

pub fn extract_debug(mut debug: ResMut<DebugTerrain>, extracted_debug: Extract<Res<DebugTerrain>>) {
    *debug = extracted_debug.clone();
}

pub fn toggle_debug(input: Res<Input<KeyCode>>, mut debug: ResMut<DebugTerrain>) {
    if input.just_pressed(KeyCode::W) {
        debug.wireframe = !debug.wireframe;
        println!(
            "Toggled the wireframe view {}.",
            if debug.wireframe { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::P) {
        debug.show_tiles = !debug.show_tiles;
        println!(
            "Toggled the tile view {}.",
            if debug.show_tiles { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::L) {
        debug.show_lod = !debug.show_lod;
        println!(
            "Toggled the lod view {}.",
            if debug.show_lod { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::U) {
        debug.show_uv = !debug.show_uv;
        println!(
            "Toggled the uv view {}.",
            if debug.show_uv { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::C) {
        debug.show_nodes = !debug.show_nodes;
        println!(
            "Toggled the node view {}.",
            if debug.show_nodes { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::Y) {
        debug.show_minmax_error = !debug.show_minmax_error;
        println!(
            "Toggled the minmax error view {}.",
            if debug.show_minmax_error { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::M) {
        debug.minmax = !debug.minmax;
        println!(
            "Toggled the minmax view {}.",
            if debug.minmax { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::D) {
        debug.mesh_morph = !debug.mesh_morph;
        println!(
            "Toggled the mesh morph {}.",
            if debug.mesh_morph { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::A) {
        debug.albedo = !debug.albedo;
        println!(
            "Toggled the albedo {}.",
            if debug.albedo { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::B) {
        debug.bright = !debug.bright;
        println!(
            "Toggled the base color to {}.",
            if debug.bright { "white" } else { "black" }
        )
    }
    if input.just_pressed(KeyCode::S) {
        debug.lighting = !debug.lighting;
        println!(
            "Toggled the lighting {}.",
            if debug.lighting { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::G) {
        debug.sample_grad = !debug.sample_grad;
        println!(
            "Toggled the texture sampling using gradients {}.",
            if debug.sample_grad { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::F) {
        debug.freeze = !debug.freeze;
        println!(
            "{} the view frustum.",
            if debug.freeze { "Froze" } else { "Unfroze" }
        )
    }
    if input.just_pressed(KeyCode::Key1) {
        debug.test1 = !debug.test1;
        println!(
            "Toggled the debug flag 1 {}.",
            if debug.test1 { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::Key2) {
        debug.test2 = !debug.test2;
        println!(
            "Toggled the debug flag 2 {}.",
            if debug.test2 { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::Key3) {
        debug.test3 = !debug.test3;
        println!(
            "Toggled the debug flag 3 {}.",
            if debug.test3 { "on" } else { "off" }
        )
    }
}

pub fn change_config(
    input: Res<Input<KeyCode>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    for mut view_config in &mut view_configs.0.values_mut() {
        if input.just_pressed(KeyCode::H) && view_config.tile_scale > 0.25 {
            view_config.tile_scale /= 2.0;
            println!("Decreased the tile scale to {}.", view_config.tile_scale);
        }
        if input.just_pressed(KeyCode::J) {
            view_config.tile_scale *= 2.0;
            println!("Increased the tile scale to {}.", view_config.tile_scale)
        }

        if input.just_pressed(KeyCode::I) {
            view_config.view_distance -= 0.25;
            println!(
                "Decreased the view distance to {}.",
                view_config.view_distance
            );
        }
        if input.just_pressed(KeyCode::O) {
            view_config.view_distance += 0.25;
            println!(
                "Increased the view distance to {}.",
                view_config.view_distance
            );
        }

        if input.just_pressed(KeyCode::N) && view_config.grid_size > 2 {
            view_config.grid_size -= 2;
            println!("Decreased the grid size to {}.", view_config.grid_size);
        }
        if input.just_pressed(KeyCode::E) {
            view_config.grid_size += 2;
            println!("Increased the grid size to {}.", view_config.grid_size);
        }
    }
}
