use bevy::prelude::*;
use bevy_terrain::prelude::*;
use bevy_terrain::preprocess_gpu::{NewPreprocessor, TerrainPreprocessPlugin};

const TILE_SIZE: u32 = 1024;
const TILE_FORMAT: FileFormat = FileFormat::PNG;
const TERRAIN_SIZE: f32 = 1024.0;
const TEXTURE_SIZE: u32 = 512;
const MIP_LEVEL_COUNT: u32 = 1;
const LOD_COUNT: u32 = 8;
const HEIGHT: f32 = 400.0 / TERRAIN_SIZE;
const NODE_ATLAS_SIZE: u32 = 1024;
const PATH: &str = "terrains/basic";

fn main() {
    let config =
        TerrainPluginConfig::with_base_attachment(BaseConfig::new(TEXTURE_SIZE, MIP_LEVEL_COUNT));

    App::new()
        .add_plugins((
            DefaultPlugins,
            TerrainPlugin { config },
            TerrainPreprocessPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    plugin_config: Res<TerrainPluginConfig>,
    asset_server: Res<AssetServer>,
) {
    let config = plugin_config.configure_terrain(
        TERRAIN_SIZE,
        LOD_COUNT,
        0.0,
        HEIGHT,
        NODE_ATLAS_SIZE,
        PATH.to_string(),
    );

    let mut terrain_bundle =
        TerrainBundle::new(config.clone(), Vec3::new(20.0, -30.0, -100.0), TERRAIN_SIZE);

    let mut preprocessor = NewPreprocessor::new();

    preprocessor.preprocess_tile(
        TileConfig {
            side: 0,
            path: format!("{PATH}/source/height.png"),
            size: TILE_SIZE,
            file_format: TILE_FORMAT,
        },
        &asset_server,
        &mut terrain_bundle.node_atlas,
    );

    commands.spawn((terrain_bundle, preprocessor));
}
