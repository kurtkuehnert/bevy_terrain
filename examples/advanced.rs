use bevy::{
    asset::{ChangeWatcher, LoadState},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::*,
};
use bevy_terrain::prelude::*;
use std::time::Duration;

const TILE_SIZE: u32 = 1024;
const TILE_FORMAT: FileFormat = FileFormat::PNG;
const TERRAIN_SIZE: f32 = 1024.0;
const TEXTURE_SIZE: u32 = 256;
const MIP_LEVEL_COUNT: u32 = 1;
const LOD_COUNT: u32 = 4;
const HEIGHT: f32 = 100.0;
const NODE_ATLAS_SIZE: u32 = 1024;
const PATH: &str = "terrains/advanced";

#[derive(AsBindGroup, TypeUuid, TypePath, Clone)]
#[uuid = "4ccc53dd-2cfd-48ba-b659-c0e1a9bc0bdb"]
pub struct TerrainMaterial {
    #[texture(0, dimension = "1d")]
    #[sampler(1)]
    gradient: Handle<Image>,
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/advanced.wgsl".into()
    }
}

fn main() {
    let config =
        TerrainPluginConfig::with_base_attachment(BaseConfig::new(TEXTURE_SIZE, MIP_LEVEL_COUNT))
            .add_attachment(AttachmentConfig::new(
                "albedo".to_string(),
                TEXTURE_SIZE,
                1,
                MIP_LEVEL_COUNT,
                AttachmentFormat::Rgb8,
            ));

    App::new()
        .insert_resource(ClearColor(Color::rgb_u8(43, 44, 47)))
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
        .add_systems(Update, (create_array_texture, toggle_camera))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    plugin_config: Res<TerrainPluginConfig>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    let gradient = asset_server.load("textures/gradient.png");
    commands.insert_resource(LoadingTexture {
        is_loaded: false,
        gradient: gradient.clone(),
    });

    let mut loader = AttachmentFromDiskLoader::new(LOD_COUNT, PATH.to_string());
    loader.add_base_attachment(
        &plugin_config,
        TileConfig {
            side: 0,
            path: format!("assets/{PATH}/source/height"),
            size: TILE_SIZE,
            file_format: TILE_FORMAT,
        },
    );
    loader.add_attachment(
        &plugin_config,
        TileConfig {
            side: 0,
            path: format!("assets/{PATH}/source/albedo.png"),
            size: TILE_SIZE,
            file_format: TILE_FORMAT,
        },
    );

    // Preprocesses the terrain data.
    // Todo: Should be commented out after the first run.
    // loader.preprocess();

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = plugin_config.configure_terrain(
        TILE_SIZE as f32 / plugin_config.leaf_node_size as f32,
        TERRAIN_SIZE,
        LOD_COUNT,
        HEIGHT,
        NODE_ATLAS_SIZE,
        PATH.to_string(),
    );

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig {
        grid_size: 16,
        node_count: 8,
        load_distance: 500.0,
        morph_distance: 8.0,
        blend_distance: 100.0,
        ..default()
    };

    // Create the terrain.
    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone()),
            loader,
            materials.add(TerrainMaterial { gradient }),
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
        transform: Transform::from_xyz(-1.0, 1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
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

#[derive(Resource)]
struct LoadingTexture {
    is_loaded: bool,
    gradient: Handle<Image>,
}

fn create_array_texture(
    asset_server: Res<AssetServer>,
    mut loading_texture: ResMut<LoadingTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    if loading_texture.is_loaded
        || asset_server.get_load_state(loading_texture.gradient.clone()) != LoadState::Loaded
    {
        return;
    }

    loading_texture.is_loaded = true;

    let image = images.get_mut(&loading_texture.gradient).unwrap();
    image.texture_descriptor.dimension = TextureDimension::D1;
}
