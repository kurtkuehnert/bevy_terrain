//! Contains a debug resource and systems controlling it to visualize different internal
//! data of the plugin.
use crate::{
    debug::camera::camera_controller,
    prelude::TerrainMaterialPlugin,
    terrain_view::{TerrainViewComponents, TerrainViewConfig},
};
use bevy::{
    asset::LoadState,
    prelude::*,
    render::{render_resource::*, Extract, RenderApp},
    transform::TransformSystem,
};

pub mod camera;

#[derive(Asset, AsBindGroup, TypePath, Clone, Default)]
pub struct DebugTerrainMaterial {}

impl Material for DebugTerrainMaterial {}

/// Adds a terrain debug config, a debug camera and debug control systems.
pub struct TerrainDebugPlugin;

impl Plugin for TerrainDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainMaterialPlugin::<DebugTerrainMaterial>::default())
            .init_resource::<DebugTerrain>()
            .init_resource::<LoadingImages>()
            .add_systems(Startup, debug_lighting)
            .add_systems(Update, (toggle_debug, change_config, finish_loading_images))
            .add_systems(
                PostUpdate,
                camera_controller.before(TransformSystem::TransformPropagate),
            );

        app.sub_app_mut(RenderApp)
            .init_resource::<DebugTerrain>()
            .add_systems(ExtractSchedule, extract_debug);
    }
}

#[derive(Clone, Resource)]
pub struct DebugTerrain {
    pub wireframe: bool,
    pub show_lod: bool,
    pub show_uv: bool,
    pub show_tiles: bool,
    pub show_quadtree: bool,
    pub show_pixels: bool,
    pub show_normals: bool,
    pub morph: bool,
    pub blend: bool,
    pub quadtree_lod: bool,
    pub albedo: bool,
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
            show_lod: false,
            show_uv: false,
            show_tiles: false,
            show_quadtree: false,
            show_pixels: false,
            show_normals: false,
            morph: true,
            blend: true,
            quadtree_lod: false,
            albedo: false,
            lighting: true,
            sample_grad: true,
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
        debug.show_lod = !debug.show_lod;
        println!(
            "Toggled the lod view {}.",
            if debug.show_lod { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyU) {
        debug.show_uv = !debug.show_uv;
        println!(
            "Toggled the uv view {}.",
            if debug.show_uv { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyY) {
        debug.show_tiles = !debug.show_tiles;
        println!(
            "Toggled the tile view {}.",
            if debug.show_tiles { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyQ) {
        debug.show_quadtree = !debug.show_quadtree;
        println!(
            "Toggled the quadtree view {}.",
            if debug.show_quadtree { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyP) {
        debug.show_pixels = !debug.show_pixels;
        println!(
            "Toggled the pixel view {}.",
            if debug.show_pixels { "on" } else { "off" }
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
    if input.just_pressed(KeyCode::KeyH) {
        debug.quadtree_lod = !debug.quadtree_lod;
        println!(
            "Toggled the quadtree lod {}.",
            if debug.quadtree_lod { "on" } else { "off" }
        )
    }
    if input.just_pressed(KeyCode::KeyA) {
        debug.albedo = !debug.albedo;
        println!(
            "Toggled the albedo {}.",
            if debug.albedo { "on" } else { "off" }
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

pub fn change_config(
    input: Res<ButtonInput<KeyCode>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    for view_config in &mut view_configs.0.values_mut() {
        if input.just_pressed(KeyCode::KeyN) {
            view_config.blend_distance -= 0.25;
            println!(
                "Decreased the blend distance to {}.",
                view_config.blend_distance
            );
        }
        if input.just_pressed(KeyCode::KeyE) {
            view_config.blend_distance += 0.25;
            println!(
                "Increased the blend distance to {}.",
                view_config.blend_distance
            );
        }

        if input.just_pressed(KeyCode::KeyI) {
            view_config.morph_distance -= 0.25;
            println!(
                "Decreased the morph distance to {}.",
                view_config.morph_distance
            );
        }
        if input.just_pressed(KeyCode::KeyO) {
            view_config.morph_distance += 0.25;
            println!(
                "Increased the morph distance to {}.",
                view_config.morph_distance
            );
        }

        if input.just_pressed(KeyCode::KeyX) && view_config.grid_size > 2 {
            view_config.grid_size -= 2;
            println!("Decreased the grid size to {}.", view_config.grid_size);
        }
        if input.just_pressed(KeyCode::KeyJ) {
            view_config.grid_size += 2;
            println!("Increased the grid size to {}.", view_config.grid_size);
        }
    }
}

pub(crate) fn debug_lighting(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 5000.0,
            ..default()
        },
        transform: Transform::from_xyz(-1.0, 1.0, -1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.insert_resource(AmbientLight {
        brightness: 100.0,
        ..default()
    });
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
        if asset_server.load_state(id) == LoadState::Loaded {
            let image = images.get_mut(id).unwrap();
            image.texture_descriptor.dimension = dimension;
            image.texture_descriptor.format = format;

            false
        } else {
            true
        }
    });
}
