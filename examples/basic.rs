use bevy::{prelude::*, reflect::TypeUuid, reflect::TypePath, render::render_resource::*};
use bevy_terrain::prelude::*;

const TERRAIN_SIZE: u32 = 1024;
const TEXTURE_SIZE: u32 = 512;
const MIP_LEVEL_COUNT: u32 = 1;
const LOD_COUNT: u32 = 4;
const HEIGHT: f32 = 200.0;
const NODE_ATLAS_SIZE: u32 = 100;
const PATH: &str = "terrain";

#[derive( TypePath, AsBindGroup, TypeUuid, Clone)]
#[uuid = "003e1d5d-241c-45a6-8c25-731dee22d820"]
pub struct TerrainMaterial {}

impl Material for TerrainMaterial {}
 

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TerrainPlugin {
            attachment_count: 2, // has to match the attachments of the terrain
        })
        .add_plugins(TerrainDebugPlugin) // enable debug settings and controls
        .add_plugins(TerrainMaterialPlugin::<TerrainMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update,toggle_camera)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    let mut preprocessor = Preprocessor::default();
    let mut loader = AttachmentFromDiskLoader::default();

    // Configure all the important properties of the terrain, as well as its attachments.
    let mut config = TerrainConfig::new(
        TERRAIN_SIZE,
        LOD_COUNT,
        HEIGHT,
        NODE_ATLAS_SIZE,
        PATH.to_string(),
    );

    config.add_base_attachment_from_disk(
        &mut preprocessor,
        &mut loader,
        BaseConfig::new(TEXTURE_SIZE, MIP_LEVEL_COUNT),
        TileConfig {
            path: "assets/terrain/source/height".to_string(),
            size: TERRAIN_SIZE,
            file_format: FileFormat::PNG,
        },
    );

    // Preprocesses the terrain data.
    // Todo: Should be commented out after the first run.
    preprocessor.preprocess(&config);

    load_node_config(&mut config);

    // Create the terrain.
    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone()),
            loader,
            materials.add(TerrainMaterial {}),
        ))
        .id();

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig {
        tile_scale: 4.0,
        grid_size: 4,
        node_count: 10,
        load_distance: 5.0,
        view_distance: 4.0,
        ..default()
    };

    // Create the view.
    let view = commands
        .spawn((
            TerrainView,
            DebugCamera::default(),
            Camera3dBundle::default(),
        ))
        .id();

    // Store the quadtree and the view config for the terrain and view.
    // This will hopefully be way nicer once the ECS can handle relations.
    let quadtree = Quadtree::from_configs(&config, &view_config);
    view_configs.insert((terrain, view), view_config);
    quadtrees.insert((terrain, view), quadtree);

    // Create a sunlight for the physical based lighting.
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 20000.0,
            ..default()
        },
        transform: Transform::from_xyz(1.0, 1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.insert_resource(AmbientLight {
        brightness: 0.2,
        ..default()
    });
}

fn toggle_camera(input: Res<Input<KeyCode>>, mut camera_query: Query<&mut DebugCamera>) {
    let mut camera = camera_query.single_mut();
    if input.just_pressed(KeyCode::T) {
        camera.active = !camera.active;
    }
}
