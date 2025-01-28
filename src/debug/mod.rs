//! Contains a debug resource and systems controlling it to visualize different internal
//! data of the plugin.
use crate::{
    debug::{camera::debug_camera_controller, orbital_camera::orbital_camera_controller},
    prelude::TileAtlas,
    terrain_data::TileTree,
    terrain_view::TerrainViewComponents,
};
use bevy::{
    prelude::*,
    render::{render_resource::*, Extract, RenderApp},
    transform::TransformSystem,
    window::PrimaryWindow,
};

mod camera;
mod orbital_camera;

pub use crate::debug::{camera::DebugCameraController, orbital_camera::OrbitalCameraController};

#[derive(Asset, AsBindGroup, TypePath, Clone, Default)]
pub struct DebugTerrainMaterial {}

impl Material for DebugTerrainMaterial {}

/// Adds a terrain debug config, a debug camera and debug control systems.
pub struct TerrainDebugPlugin;

impl Plugin for TerrainDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugTerrain>()
            .init_resource::<LoadingImages>()
            .add_systems(Startup, (debug_lighting, debug_window))
            .add_systems(
                Update,
                (
                    toggle_debug,
                    update_view_parameter,
                    finish_loading_images,
                    orbital_camera_controller,
                ),
            )
            .add_systems(
                PostUpdate,
                debug_camera_controller.before(TransformSystem::TransformPropagate),
            );

        app.sub_app_mut(RenderApp)
            .init_resource::<DebugTerrain>()
            .add_systems(ExtractSchedule, extract_debug);
    }
}

#[derive(Clone, Resource)]
pub struct DebugTerrain {
    pub wireframe: bool,
    pub show_data_lod: bool,
    pub show_geometry_lod: bool,
    pub show_tile_tree: bool,
    pub show_pixels: bool,
    pub show_uv: bool,
    pub show_normals: bool,
    pub morph: bool,
    pub blend: bool,
    pub tile_tree_lod: bool,
    pub lighting: bool,
    pub sample_grad: bool,
    pub high_precision: bool,
    pub freeze: bool,
    pub test1: bool,
    pub test2: bool,
    pub test3: bool,
}

impl Default for DebugTerrain {
    fn default() -> Self {
        Self {
            wireframe: false,
            show_data_lod: false,
            show_geometry_lod: false,
            show_tile_tree: false,
            show_pixels: false,
            show_uv: false,
            show_normals: false,
            morph: true,
            blend: true,
            tile_tree_lod: false,
            lighting: true,
            sample_grad: true,
            high_precision: true,
            freeze: false,
            test1: false,
            test2: false,
            test3: false,
        }
    }
}

pub fn extract_debug(mut debug: ResMut<DebugTerrain>, extracted_debug: Extract<Res<DebugTerrain>>) {
    *debug = extracted_debug.clone();
}

pub fn toggle_debug(input: Res<ButtonInput<KeyCode>>, mut debug: ResMut<DebugTerrain>) {
    if input.just_pressed(KeyCode::KeyW) {
        debug.wireframe = !debug.wireframe;
        println!(
            "Toggled the wireframe view {}.",
            if debug.wireframe { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyL) {
        debug.show_data_lod = !debug.show_data_lod;
        println!(
            "Toggled the terrain data LOD view {}.",
            if debug.show_data_lod { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyY) {
        debug.show_geometry_lod = !debug.show_geometry_lod;
        println!(
            "Toggled the terrain geometry LOD view {}.",
            if debug.show_geometry_lod { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyQ) {
        debug.show_tile_tree = !debug.show_tile_tree;
        println!(
            "Toggled the tile tree LOD view {}.",
            if debug.show_tile_tree { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyP) {
        debug.show_pixels = !debug.show_pixels;
        println!(
            "Toggled the pixel view {}.",
            if debug.show_pixels { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyU) {
        debug.show_uv = !debug.show_uv;
        println!(
            "Toggled the uv view {}.",
            if debug.show_uv { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyB) {
        debug.show_normals = !debug.show_normals;
        println!(
            "Toggled the normals view {}.",
            if debug.show_normals { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyM) {
        debug.morph = !debug.morph;
        println!(
            "Toggled morphing {}.",
            if debug.morph { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyK) {
        debug.blend = !debug.blend;
        println!(
            "Toggled blending {}.",
            if debug.blend { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyZ) {
        debug.tile_tree_lod = !debug.tile_tree_lod;
        println!(
            "Toggled tile tree lod {}.",
            if debug.tile_tree_lod { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyS) {
        debug.lighting = !debug.lighting;
        println!(
            "Toggled the lighting {}.",
            if debug.lighting { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyG) {
        debug.sample_grad = !debug.sample_grad;
        println!(
            "Toggled the texture sampling using gradients {}.",
            if debug.sample_grad { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyH) {
        debug.high_precision = !debug.high_precision;
        println!(
            "Toggled high precision coordinates {}.",
            if debug.high_precision { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyF) {
        debug.freeze = !debug.freeze;
        println!(
            "{} the view frustum.",
            if debug.freeze { "Froze" } else { "Unfroze" }
        )
    }
    if input.just_pressed(KeyCode::Digit1) {
        debug.test1 = !debug.test1;
        println!(
            "Toggled the debug flag 1 {}.",
            if debug.test1 { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::Digit2) {
        debug.test2 = !debug.test2;
        println!(
            "Toggled the debug flag 2 {}.",
            if debug.test2 { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::Digit3) {
        debug.test3 = !debug.test3;
        println!(
            "Toggled the debug flag 3 {}.",
            if debug.test3 { "on" } else { "off" }
        )
    }
}

pub fn update_view_parameter(
    input: Res<ButtonInput<KeyCode>>,
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
    tile_atlases: Query<&TileAtlas>,
) {
    for (&(terrain, _), tile_tree) in tile_trees.iter_mut() {
        let Ok(tile_atlas) = tile_atlases.get(terrain) else {
            return;
        };

        let scale = tile_atlas.model.scale();

        if input.pressed(KeyCode::ShiftLeft) && input.just_pressed(KeyCode::Equal) {
            tile_tree.height_scale += 0.1;
        }
        if input.just_pressed(KeyCode::Minus) {
            tile_tree.height_scale -= 0.1;
        }
        if input.just_pressed(KeyCode::KeyN) {
            tile_tree.blend_distance -= 0.25 * scale;
            println!(
                "Decreased the blend distance to {}.",
                tile_tree.blend_distance / scale
            );
        }
        if input.just_pressed(KeyCode::KeyE) {
            tile_tree.blend_distance += 0.25 * scale;
            println!(
                "Increased the blend distance to {}.",
                tile_tree.blend_distance / scale
            );
        }

        if input.just_pressed(KeyCode::KeyI) {
            tile_tree.morph_distance -= 0.25 * scale;
            println!(
                "Decreased the morph distance to {}.",
                tile_tree.morph_distance / scale
            );
        }
        if input.just_pressed(KeyCode::KeyO) {
            tile_tree.morph_distance += 0.25 * scale;
            println!(
                "Increased the morph distance to {}.",
                tile_tree.morph_distance / scale
            );
        }

        if input.just_pressed(KeyCode::KeyX) && tile_tree.grid_size > 2 {
            tile_tree.grid_size -= 2;
            println!("Decreased the grid size to {}.", tile_tree.grid_size);
        }
        if input.just_pressed(KeyCode::KeyJ) {
            tile_tree.grid_size += 2;
            println!("Increased the grid size to {}.", tile_tree.grid_size);
        }
    }
}

pub(crate) fn debug_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            ..default()
        },
        Transform::from_xyz(-1.0, 1.0, -3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(AmbientLight {
        brightness: 100.0,
        ..default()
    });
}

pub fn debug_window(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = window.single_mut();
    window.cursor_options.visible = true; // false;
}

#[derive(Resource, Default)]
pub struct LoadingImages(Vec<(AssetId<Image>, TextureDimension, TextureFormat)>);

impl LoadingImages {
    pub fn load_image(
        &mut self,
        handle: &Handle<Image>,
        dimension: TextureDimension,
        format: TextureFormat,
    ) -> &mut Self {
        self.0.push((handle.id(), dimension, format));
        self
    }
}

fn finish_loading_images(
    asset_server: Res<AssetServer>,
    mut loading_images: ResMut<LoadingImages>,
    mut images: ResMut<Assets<Image>>,
) {
    loading_images.0.retain(|&(id, dimension, format)| {
        if asset_server.load_state(id).is_loaded() {
            let image = images.get_mut(id).unwrap();
            image.texture_descriptor.dimension = dimension;
            image.texture_descriptor.format = format;

            false
        } else {
            true
        }
    });
}
