use bevy::prelude::*;
use bevy_terrain::prelude::*;
use bevy_terrain::preprocess_gpu::preprocessor::{PreprocessDataset, Preprocessor};
use bevy_terrain::preprocess_gpu::TerrainPreprocessPlugin;

const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 4;
const NODE_ATLAS_SIZE: u32 = 1024;
const PATH: &str = "terrains/advanced";

fn main() {
    let config = TerrainPluginConfig::with_base_attachment(BaseConfig::new(TEXTURE_SIZE, 1))
        .add_attachment(AttachmentConfig::new(
            "albedo".to_string(),
            TEXTURE_SIZE,
            1,
            1,
            AttachmentFormat::Rgba8,
        ));

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
        0.0,
        LOD_COUNT,
        0.0,
        0.0,
        NODE_ATLAS_SIZE,
        PATH.to_string(),
    );

    let mut terrain_bundle = TerrainBundle::new(config.clone(), Vec3::ZERO, 0.0);

    let mut preprocessor = Preprocessor::new(PATH.to_string());

    preprocessor.preprocess_tile(
        PreprocessDataset {
            attachment_index: 0,
            path: format!("{PATH}/source/height.png"),
        },
        &asset_server,
        &mut terrain_bundle.node_atlas,
    );
    preprocessor.preprocess_tile(
        PreprocessDataset {
            attachment_index: 1,
            path: format!("{PATH}/source/albedo.png"),
        },
        &asset_server,
        &mut terrain_bundle.node_atlas,
    );

    commands.spawn((terrain_bundle, preprocessor));
}
