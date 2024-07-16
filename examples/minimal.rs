use bevy::math::DVec3;
use bevy::prelude::*;
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/planar";
const TERRAIN_SIZE: f64 = 1000.0;
const HEIGHT: f32 = 250.0;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 4;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            TerrainPlugin,
            TerrainMaterialPlugin::<DebugTerrainMaterial>::default(),
            TerrainDebugPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<DebugTerrainMaterial>>,
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Configure all the important properties of the terrain, as well as its attachments.
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        model: TerrainModel::planar(DVec3::new(0.0, -100.0, 0.0), TERRAIN_SIZE, 0.0, HEIGHT),
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
            TerrainBundle::new(config.clone()),
            materials.add(DebugTerrainMaterial::default()),
        ))
        .id();

    let view = commands
        .spawn((TerrainView, DebugCameraBundle::default()))
        .id();

    initialize_terrain_view(
        terrain,
        view,
        &config,
        view_config,
        &mut tile_trees,
        &mut view_configs,
    );

    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::from_length(10.0)),
        transform: Transform::from_translation(Vec3::new(
            TERRAIN_SIZE as f32 / 2.0,
            100.0,
            TERRAIN_SIZE as f32 / 2.0,
        )),
        ..default()
    });
}
