use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::*,
};
use bevy_terrain::prelude::*;

const TERRAIN_SIZE: f32 = 8.0 * 507.5;
const TEXTURE_SIZE: u32 = 512;
const MIP_LEVEL_COUNT: u32 = 1;
const LOD_COUNT: u32 = 4;
const HEIGHT: f32 = 2000.0 / TERRAIN_SIZE;
const NODE_ATLAS_SIZE: u32 = 1024;
const PATH: &str = "terrains/basic";

#[derive(Asset, AsBindGroup, TypeUuid, TypePath, Clone)]
#[uuid = "003e1d5d-241c-45a6-8c25-731dee22d820"]
pub struct TerrainMaterial {}

impl Material for TerrainMaterial {}

fn main() {
    let config =
        TerrainPluginConfig::with_base_attachment(BaseConfig::new(TEXTURE_SIZE, MIP_LEVEL_COUNT));

    App::new()
        .add_plugins((
            DefaultPlugins,
            TerrainPlugin { config },
            TerrainDebugPlugin, // enable debug settings and controls
            TerrainMaterialPlugin::<TerrainMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_camera)
        .run();
}

fn setup(
    mut commands: Commands,
    plugin_config: Res<TerrainPluginConfig>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    // Configure all the important properties of the terrain, as well as its attachments.
    let config = plugin_config.configure_terrain(
        TERRAIN_SIZE,
        LOD_COUNT,
        0.0,
        HEIGHT,
        NODE_ATLAS_SIZE,
        PATH.to_string(),
    );

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig {
        grid_size: 16,
        quadtree_size: 8,
        load_distance: 3.0,
        morph_distance: 8.0,
        blend_distance: 1.5,
        ..default()
    };

    // Create the terrain.
    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone(), Vec3::new(20.0, -30.0, -100.0), TERRAIN_SIZE),
            materials.add(TerrainMaterial {}),
        ))
        .id();

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
