use bevy::{prelude::*, reflect::TypeUuid, render::render_resource::*};
use bevy_terrain::prelude::*;

const TERRAIN_SIZE: u32 = 1024;
const LOD_COUNT: u32 = 10;
const CHUNK_SIZE: u32 = 128;
const HEIGHT: f32 = 200.0;
const NODE_ATLAS_SIZE: u32 = 500;
const PATH: &str = "terrain";

#[derive(AsBindGroup, TypeUuid, Clone)]
#[uuid = "003e1d5d-241c-45a6-8c25-731dee22d820"]
pub struct TerrainMaterial {}

impl Material for TerrainMaterial {}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(TerrainPipelineConfig {
            attachment_count: 2, // has to match the attachments of the terrain
        })
        .add_plugin(TerrainPlugin)
        .add_plugin(TerrainDebugPlugin) // enable debug settings and controls
        .add_plugin(TerrainMaterialPlugin::<TerrainMaterial>::default())
        .add_startup_system(setup)
        .add_system(toggle_camera)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    let mut preprocessor = Preprocessor::default();
    let mut from_disk_loader = AttachmentFromDiskLoader::default();

    // Configure all the important properties of the terrain, as well as its attachments.
    let mut config = TerrainConfig::new(
        TERRAIN_SIZE,
        CHUNK_SIZE,
        LOD_COUNT,
        HEIGHT,
        NODE_ATLAS_SIZE,
        PATH.to_string(),
    );

    config.add_base_attachment(
        &mut preprocessor,
        &mut from_disk_loader,
        BaseConfig {
            center_size: CHUNK_SIZE,
            file_format: FileFormat::DTM,
        },
        TileConfig {
            path: "assets/terrain/source/height".to_string(),
            size: TERRAIN_SIZE,
            file_format: FileFormat::PNG,
        },
    );

    // Create the terrain.
    let terrain = commands
        .spawn_bundle(TerrainBundle::new(config.clone()))
        .insert(from_disk_loader)
        .insert(materials.add(TerrainMaterial {}))
        .id();

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig::new(&config, 10, 5.0, 3.0, 1.0, 0.2, 0.2, 0.2);

    // Create the view.
    let view = commands
        .spawn()
        .insert(DebugCamera::default())
        .insert_bundle(Camera3dBundle::default())
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
            illuminance: 20000.0,
            ..default()
        },
        transform: Transform::from_xyz(1.0, 1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Preprocesses the terrain data.
    // Todo: Should be commented out after the first run.
    preprocessor.preprocess(&config);
}

fn toggle_camera(input: Res<Input<KeyCode>>, mut camera_query: Query<&mut DebugCamera>) {
    let mut camera = camera_query.single_mut();
    if input.just_pressed(KeyCode::T) {
        camera.active = !camera.active;
    }
}
