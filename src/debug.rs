use crate::{TerrainViewComponents, TerrainViewConfig};
use bevy::{prelude::*, render::RenderWorld};

#[derive(Clone)]
pub struct DebugTerrain {
    pub wireframe: bool,

    pub show_patches: bool,
    pub show_lod: bool,
    pub show_uv: bool,

    pub circular_lod: bool,
    pub mesh_morph: bool,

    pub albedo: bool,
    pub bright: bool,
    pub lighting: bool,
}

impl Default for DebugTerrain {
    fn default() -> Self {
        Self {
            wireframe: false,
            show_patches: false,
            show_lod: false,
            show_uv: false,
            circular_lod: true,
            mesh_morph: true,
            albedo: false,
            bright: false,
            lighting: true,
        }
    }
}

pub fn toggle_debug(input: Res<Input<KeyCode>>, mut debug: ResMut<DebugTerrain>) {
    if input.just_pressed(KeyCode::W) {
        debug.wireframe = !debug.wireframe;
    }

    if input.just_pressed(KeyCode::P) {
        debug.show_patches = !debug.show_patches;
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

    if input.just_pressed(KeyCode::A) {
        debug.albedo = !debug.albedo;
    }
    if input.just_pressed(KeyCode::B) {
        debug.bright = !debug.bright;
    }
    if input.just_pressed(KeyCode::S) {
        debug.lighting = !debug.lighting;
    }
}

pub fn extract_debug(mut render_world: ResMut<RenderWorld>, debug: Res<DebugTerrain>) {
    render_world.insert_resource(debug.clone());
}

pub fn change_config(
    input: Res<Input<KeyCode>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    for config in view_configs.values_mut() {
        if input.just_pressed(KeyCode::H) && config.patch_size > 2 {
            config.change_patch_size(config.patch_size - 2);
        }
        if input.just_pressed(KeyCode::J) {
            config.change_patch_size(config.patch_size + 2);
        }

        if input.just_pressed(KeyCode::X) && config.patch_scale > 0.25 {
            config.change_patch_scale(config.patch_scale - 0.25);
        }
        if input.just_pressed(KeyCode::Q) {
            config.change_patch_scale(config.patch_scale + 0.25);
        }

        if input.just_pressed(KeyCode::I) {
            config.view_distance *= 0.95;
        }
        if input.just_pressed(KeyCode::O) {
            config.view_distance *= 1.05;
        }
    }
}
