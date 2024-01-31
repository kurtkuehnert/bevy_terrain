use bevy::prelude::*;
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/planar";
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 4;

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
        ..default()
    }
    .add_attachment(AttachmentConfig {
        name: "height".to_string(),
        texture_size: TEXTURE_SIZE,
        border_size: 2,
        format: AttachmentFormat::R16,
        ..default()
    })
    .add_attachment(AttachmentConfig {
        name: "albedo".to_string(),
        texture_size: TEXTURE_SIZE,
        border_size: 2,
        format: AttachmentFormat::Rgba8,
        ..default()
    });

    let mut terrain_bundle = TerrainBundle::new(config, Vec3::ZERO, 0.0);

    let mut preprocessor = Preprocessor::new(PATH.to_string());

    preprocessor.clear_attachment(0, &mut terrain_bundle.node_atlas);
    preprocessor.clear_attachment(1, &mut terrain_bundle.node_atlas);
    preprocessor.preprocess_tile(
        PreprocessDataset {
            attachment_index: 0,
            path: format!("{PATH}/source/height.png"),
            ..default()
        },
        &asset_server,
        &mut terrain_bundle.node_atlas,
    );
    preprocessor.preprocess_tile(
        PreprocessDataset {
            attachment_index: 1,
            path: format!("{PATH}/source/albedo.png"),
            ..default()
        },
        &asset_server,
        &mut terrain_bundle.node_atlas,
    );

    commands.spawn((terrain_bundle, preprocessor));
}
