use bevy::{prelude::*, reflect::TypePath, render::render_resource::*};
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/planar";
const TERRAIN_SIZE: f32 = 1000.0;
const HEIGHT: f32 = 500.0 / TERRAIN_SIZE;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 4;

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct TerrainMaterial {}

impl Material for TerrainMaterial {}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            TerrainPlugin,
            TerrainDebugPlugin, // enable debug settings and controls
            TerrainMaterialPlugin::<TerrainMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    // Configure all the important properties of the terrain, as well as its attachments.
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        max_height: HEIGHT,
        path: PATH.to_string(),
        ..default()
    }
    .add_attachment(AttachmentConfig {
        name: "height".to_string(),
        texture_size: TEXTURE_SIZE,
        border_size: 2,
        mip_level_count: 4,
        format: AttachmentFormat::R16,
    });

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig::default();

    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone(), default(), TERRAIN_SIZE),
            materials.add(TerrainMaterial {}),
        ))
        .id();

    let view = commands.spawn((TerrainView, DebugCamera::default())).id();

    initialize_terrain_view(
        terrain,
        view,
        &config,
        view_config,
        &mut quadtrees,
        &mut view_configs,
    );
}
