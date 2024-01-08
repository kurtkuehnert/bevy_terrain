use bevy::prelude::*;
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/spherical";
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 5;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TerrainPlugin, TerrainPreprocessPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        path: PATH.to_string(),
        node_atlas_size: 2048,
        ..default()
    }
    .add_attachment(AttachmentConfig::new(
        "height".to_string(),
        TEXTURE_SIZE,
        2,
        AttachmentFormat::R16,
    ));

    let mut terrain_bundle = TerrainBundle::new(config, Vec3::ZERO, 0.0);

    let mut preprocessor = Preprocessor::new(PATH.to_string());

    for side in 0..6 {
        preprocessor.preprocess_tile(
            PreprocessDataset {
                attachment_index: 0,
                path: format!("{PATH}/source/height/face{side}.png"),
                side,
            },
            &asset_server,
            &mut terrain_bundle.node_atlas,
        );
    }

    commands.spawn((terrain_bundle, preprocessor));
}
