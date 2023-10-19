use bevy::asset::LoadState;
use bevy::{
    asset::ChangeWatcher,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::*,
};
use bevy_terrain::prelude::*;
use std::time::Duration;

const TERRAIN_SIZE: f32 = 0.0;
const RADIUS: f32 = 50.0;
const TEXTURE_SIZE: u32 = 512;
const MIP_LEVEL_COUNT: u32 = 1;
const LOD_COUNT: u32 = 4;
const HEIGHT: f32 = 200.0;
const NODE_ATLAS_SIZE: u32 = 100;
const PATH: &str = "terrain";

#[derive(AsBindGroup, TypeUuid, TypePath, Clone)]
#[uuid = "003e1d5d-241c-45a6-8c25-731dee22d820"]
pub struct TerrainMaterial {
    #[texture(0, dimension = "cube")]
    cube_map: Handle<Image>,
    #[texture(1, dimension = "1d")]
    gradient: Handle<Image>,
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/spherical.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/spherical.wgsl".into()
    }
}

fn main() {
    let config =
        TerrainPluginConfig::with_base_attachment(BaseConfig::new(TEXTURE_SIZE, MIP_LEVEL_COUNT));

    App::new()
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                watch_for_changes: ChangeWatcher::with_delay(Duration::from_millis(200)), // enable hot reloading for shader easy customization
                ..default()
            }),
            TerrainPlugin { config },
            TerrainDebugPlugin, // enable debug settings and controls
            TerrainMaterialPlugin::<TerrainMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_camera, create_array_texture))
        .run();
}

fn setup(
    mut commands: Commands,
    plugin_config: Res<TerrainPluginConfig>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    let cube_map = asset_server.load("textures/test_stacked.png");
    let gradient = asset_server.load("textures/gradient.png");

    commands.insert_resource(LoadingTextures {
        is_loaded: false,
        cube_map: cube_map.clone(),
        gradient: gradient.clone(),
    });

    let mut loader = AttachmentFromDiskLoader::new(LOD_COUNT, PATH.to_string());
    loader.add_base_attachment(
        &plugin_config,
        TileConfig {
            path: "assets/terrain/source/height".to_string(),
            size: TERRAIN_SIZE as u32,
            file_format: FileFormat::PNG,
        },
    );

    // Preprocesses the terrain data.
    // Todo: Should be commented out after the first run.
    // loader.preprocess();

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = plugin_config.configure_terrain(
        TERRAIN_SIZE,
        RADIUS,
        LOD_COUNT,
        HEIGHT,
        NODE_ATLAS_SIZE,
        PATH.to_string(),
    );

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig {
        tile_scale: 16.0,
        grid_size: 8,
        node_count: 10,
        load_distance: 5.0,
        view_distance: 8.0,
        ..default()
    };

    // Create the terrain.
    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone()),
            loader,
            materials.add(TerrainMaterial { cube_map, gradient }),
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

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 5.0 })),
        ..default()
    });
}

fn toggle_camera(input: Res<Input<KeyCode>>, mut camera_query: Query<&mut DebugCamera>) {
    let mut camera = camera_query.single_mut();
    if input.just_pressed(KeyCode::T) {
        camera.active = !camera.active;
    }
}

#[derive(Resource)]
struct LoadingTextures {
    is_loaded: bool,
    cube_map: Handle<Image>,
    gradient: Handle<Image>,
}

fn create_array_texture(
    asset_server: Res<AssetServer>,
    mut loading_textures: ResMut<LoadingTextures>,
    mut images: ResMut<Assets<Image>>,
) {
    if loading_textures.is_loaded
        || asset_server.get_load_state(loading_textures.cube_map.clone()) != LoadState::Loaded
        || asset_server.get_load_state(loading_textures.gradient.clone()) != LoadState::Loaded
    {
        return;
    }

    loading_textures.is_loaded = true;

    let image = images.get_mut(&loading_textures.cube_map).unwrap();

    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });

    // Create a new array texture asset from the loaded texture.
    let array_layers = 6;
    image.reinterpret_stacked_2d_as_array(array_layers);

    let image = images.get_mut(&loading_textures.gradient).unwrap();
    image.texture_descriptor.dimension = TextureDimension::D1;
}
