use bevy::{prelude::*, render::render_resource::*};
use bevy_terrain::{prelude::*, preprocess::prelude::*};

const TERRAIN_SIZE: u32 = 1024;
const LOD_COUNT: u32 = 5;
const CHUNK_SIZE: u32 = 128;
const HEIGHT: f32 = 200.0;
const NODE_ATLAS_SIZE: u32 = 300;

fn main() {
    // Should only be run once. Comment out after the first run.
    preprocess();

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(TerrainPipelineConfig {
            attachment_count: 3, // has to match the attachments of the terrain
            ..default()
        })
        .add_plugin(TerrainPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    // Configure all the important properties of the terrain, as well as its attachments.
    let mut config = TerrainConfig::new(
        TERRAIN_SIZE,
        CHUNK_SIZE,
        LOD_COUNT,
        HEIGHT,
        NODE_ATLAS_SIZE,
        "terrain/".to_string(),
    );
    let mut from_disk_loader = AttachmentFromDiskLoader::default();

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

    // Create the terrain.
    let terrain = commands
        .spawn_bundle(TerrainBundle::new(config.clone()))
        .insert(from_disk_loader)
        .id();

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig::new(&config, 10, 5.0, 3.0, 10.0, 0.2, 0.2, 0.2);

    // Create the view.
    let view = commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(-200.0, 500.0, -200.0)
                .looking_at(Vec3::new(500.0, 0.0, 500.0), Vec3::Y),
            ..default()
        })
        .insert(TerrainView)
        .id();

    // Store the quadtree and the view config for the terrain and view.
    // This will hopefully be way nicer once the ECS can handle relations.
    let quadtree = Quadtree::from_configs(&config, &view_config);
    view_configs.insert((terrain, view), view_config);
    quadtrees.insert((terrain, view), quadtree);

    // Create a sunlight for the physical based lighting.
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            ..default()
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
