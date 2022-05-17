use bevy::prelude::*;
use bevy::render::RenderWorld;

#[derive(Clone)]
pub struct DebugTerrain {
    pub albedo: bool,
    pub show_patches: bool,
    pub show_lod: bool,
}

impl Default for DebugTerrain {
    fn default() -> Self {
        Self {
            albedo: true,
            show_patches: false,
            show_lod: false,
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
}
pub fn extract_debug(mut render_world: ResMut<RenderWorld>, debug: Res<DebugTerrain>) {
    render_world.insert_resource(debug.clone());
}
