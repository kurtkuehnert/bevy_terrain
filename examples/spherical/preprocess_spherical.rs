use bevy::prelude::*;
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/spherical";
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 8;

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
        ..default() //  })
                    //  .add_attachment(AttachmentConfig {
                    //      name: "height".to_string(),
                    //      texture_size: TEXTURE_SIZE,
                    //      border_size: 2,
                    //      format: AttachmentFormat::R16,
                    //      ..default()
    });

    let mut terrain_bundle = TerrainBundle::new(config, Vec3::ZERO, 0.0);

    let mut preprocessor = Preprocessor::new(PATH.to_string());

    preprocessor.clear_attachment(0, &mut terrain_bundle.node_atlas);
    //  preprocessor.clear_attachment(1, &mut terrain_bundle.node_atlas);
    preprocessor.preprocess_spherical(
        SphericalDataset {
            attachment_index: 0,
            paths: (0..6)
                .map(|side| format!("{PATH}/source/height/face{side}.tif"))
                .collect(),
            lod_range: 6..LOD_COUNT,
        },
        &asset_server,
        &mut terrain_bundle.node_atlas,
    );
    preprocessor.preprocess_tile(
        PreprocessDataset {
            attachment_index: 0,
            path: format!("{PATH}/source/height/face2.tif"),
            side: 2,
            top_left: Vec2::new(0.0, 0.0),
            bottom_right: Vec2::new(1.0, 1.0),
            lod_range: 2..LOD_COUNT,
        },
        &asset_server,
        &mut terrain_bundle.node_atlas,
    );
    preprocessor.preprocess_tile(
        PreprocessDataset {
            attachment_index: 0,
            path: format!("{PATH}/source/height/200m.tif"),
            side: 2,
            top_left: Vec2::new(0.2077404, 0.4357290),
            bottom_right: Vec2::new(0.3284694, 0.5636175),
            lod_range: 2..LOD_COUNT,
        },
        &asset_server,
        &mut terrain_bundle.node_atlas,
    );

    commands.spawn((terrain_bundle, preprocessor));
}
