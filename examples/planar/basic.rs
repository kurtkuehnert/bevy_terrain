use bevy::prelude::*;
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/planar";
const TERRAIN_SIZE: f32 = 1000.0;
const HEIGHT: f32 = 0.0;
// 250.0 / TERRAIN_SIZE;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 4;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TerrainPlugin, TerrainDebugPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<DebugTerrainMaterial>>,
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
            TerrainBundle::new(config.clone(), Vec3::new(0.0, -100.0, 0.0), TERRAIN_SIZE),
            materials.add(DebugTerrainMaterial::default()),
        ))
        .id();

    let view = commands.spawn((TerrainView, DebugCamera2d::default())).id();

    initialize_terrain_view(
        terrain,
        view,
        &config,
        view_config,
        &mut quadtrees,
        &mut view_configs,
    );
}
