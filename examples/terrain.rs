use bevy::{prelude::*, render::render_resource::*};
use bevy_terrain::{
    attachment_loader::AttachmentFromDiskLoader,
    bundles::TerrainBundle,
    preprocess::{density::preprocess_density, preprocess_tiles, ImageFormat},
    quadtree::Quadtree,
    render::TerrainPipelineConfig,
    terrain::TerrainConfig,
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
    TerrainPlugin,
};

const TERRAIN_SIZE: u32 = 1024;
const LOD_COUNT: u32 = 5;
const CHUNK_SIZE: u32 = 128;
const HEIGHT: f32 = 200.0;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .insert_resource(TerrainPipelineConfig {
            attachment_count: 3,
            ..default()
        })
        .add_plugin(TerrainPlugin)
        .add_startup_system(setup);

    // Should only be run once. Comment out after the first run.
    preprocess();

    app.run()
}

fn setup(
    mut commands: Commands,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut terrain_view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    let mut from_disk_loader = AttachmentFromDiskLoader::default();
    let mut config = TerrainConfig::new(CHUNK_SIZE, LOD_COUNT, HEIGHT, "terrain/".to_string());

    config.add_attachment_from_disk(
        &mut from_disk_loader,
        "height",
        TextureFormat::R16Unorm,
        CHUNK_SIZE,
        2,
    );
    config.add_attachment_from_disk(
        &mut from_disk_loader,
        "density",
        TextureFormat::R16Unorm,
        CHUNK_SIZE,
        0,
    );
    config.add_attachment_from_disk(
        &mut from_disk_loader,
        "albedo",
        TextureFormat::Rgba8UnormSrgb,
        2 * CHUNK_SIZE,
        1,
    );

    let terrain = commands
        .spawn_bundle(TerrainBundle::new(config.clone()))
        .insert(from_disk_loader)
        .id();

    let view = commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(-200.0, 500.0, -200.0)
                .looking_at(Vec3::new(500.0, 0.0, 500.0), Vec3::Y),
            ..default()
        })
        .insert(TerrainView)
        .id();

    let view_config = TerrainViewConfig::new(TERRAIN_SIZE, 3.0, 10.0, 0.5);
    let quadtree = Quadtree::new(&config, &view_config);

    terrain_view_configs.insert((terrain, view), view_config);
    quadtrees.insert((terrain, view), quadtree);

    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::default(),
            illuminance: 10000.0,
            shadows_enabled: false,
            shadow_projection: Default::default(),
            shadow_depth_bias: 0.0,
            shadow_normal_bias: 0.0,
        },
        transform: Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
        ..default()
    });
}

fn preprocess() {
    preprocess_tiles(
        "assets/terrain/source/height",
        "assets/terrain/data/height",
        0,
        LOD_COUNT,
        (0, 0),
        TERRAIN_SIZE,
        CHUNK_SIZE,
        2,
        ImageFormat::LUMA16,
    );

    preprocess_density(
        "assets/terrain/data/height",
        "assets/terrain/data/density",
        LOD_COUNT,
        (0, 0),
        (9, 9),
        CHUNK_SIZE,
        2,
        HEIGHT,
    );

    preprocess_tiles(
        "assets/terrain/source/albedo.png",
        "assets/terrain/data/albedo",
        0,
        LOD_COUNT,
        (0, 0),
        2 * TERRAIN_SIZE,
        2 * CHUNK_SIZE,
        1,
        ImageFormat::RGB,
    );
}
