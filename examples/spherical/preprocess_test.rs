use bevy::prelude::*;
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/test";
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
            ..default()
        });

    let mut node_atlas = NodeAtlas::from_config(&config);

    let preprocessor = Preprocessor::new()
        .clear_attachment(0, &mut node_atlas)
        .preprocess_spherical(
            SphericalDataset {
                attachment_index: 0,
                paths: (0..6)
                    .map(|side| format!("{PATH}/source/height/face{side}.png"))
                    .collect(),
                lod_range: 0..1,
            },
            &asset_server,
            &mut node_atlas,
        )
        .preprocess_tile(PreprocessDataset {
            attachment_index: 0,
            path: format!("{PATH}/source/height/200m.tif"),
            side: 2,
            top_left: Vec2::new(0.2077404, 0.4357290),
            bottom_right: Vec2::new(0.3284694, 0.5636175),
            lod_range: 0..LOD_COUNT,
        }, &asset_server, &mut node_atlas);

    commands.spawn((Terrain, node_atlas, preprocessor));
}
