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
    .add_attachment(AttachmentConfig {
        name: "height".to_string(),
        texture_size: TEXTURE_SIZE,
        border_size: 2,
        format: AttachmentFormat::R16,
        ..default()
    });
    //.add_attachment(AttachmentConfig {
    //   name: "height2".to_string(),
    //   texture_size: TEXTURE_SIZE,
    //   border_size: 2,
    //   format: AttachmentFormat::R16,
    //   ..default()
    //);

    let mut terrain_bundle = TerrainBundle::new(config, Vec3::ZERO, 0.0);

    let mut preprocessor = Preprocessor::new(PATH.to_string());

    preprocessor.preprocess_spherical(
        PreprocessDataset {
            attachment_index: 0,
            path: PATH.to_string(),
            ..default()
        },
        &asset_server,
        &mut terrain_bundle.node_atlas,
    );

    // for side in 0..6 {
    //     preprocessor.preprocess_tile(
    //         PreprocessDataset {
    //             attachment_index: 1,
    //             path: "terrains/spherical/source/height/200m.tif".to_string(),
    //             side,
    //             top_left: Vec2::new(0.0, 0.0),
    //             bottom_right: Vec2::new(0.0, 0.0),
    //         },
    //         &asset_server,
    //         &mut terrain_bundle.node_atlas,
    //     );
    // }
    //
    // preprocessor.preprocess_tile(
    //     PreprocessDataset {
    //         attachment_index: 1,
    //         path: "terrains/spherical/source/height/200m.tif".to_string(),
    //         side: 2,
    //         top_left: Vec2::new(0.2077404, 0.4357290),
    //         bottom_right: Vec2::new(0.3284694, 0.5636175),
    //     },
    //     &asset_server,
    //     &mut terrain_bundle.node_atlas,
    // );

    commands.spawn((terrain_bundle, preprocessor));
}
