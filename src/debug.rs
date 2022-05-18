use bevy::prelude::*;
use bevy::render::RenderWorld;

#[derive(Clone)]
pub struct DebugTerrain {
    pub albedo: bool,
    pub show_patches: bool,
    pub show_lod: bool,
    pub show_nodes: bool,
    pub color: bool,
    pub lighting: bool,
}

impl Default for DebugTerrain {
    fn default() -> Self {
        Self {
            albedo: true,
            show_patches: false,
            show_lod: false,
            show_nodes: false,
            color: true,
            lighting: true,
        }
    }
}

pub fn toggle_debug_system(input: Res<Input<KeyCode>>, mut debug: ResMut<DebugTerrain>) {
    if input.just_pressed(KeyCode::A) {
        debug.albedo = !debug.albedo;
    }
    if input.just_pressed(KeyCode::P) {
        debug.show_patches = !debug.show_patches;
    }
    if input.just_pressed(KeyCode::L) {
        debug.show_lod = !debug.show_lod;
    }
    if input.just_pressed(KeyCode::N) {
        debug.show_nodes = !debug.show_nodes;
    }
    if input.just_pressed(KeyCode::C) {
        debug.color = !debug.color;
    }
    if input.just_pressed(KeyCode::S) {
        debug.lighting = !debug.lighting;
    }
}

pub fn extract_debug(mut render_world: ResMut<RenderWorld>, debug: Res<DebugTerrain>) {
    render_world.insert_resource(debug.clone());
}
