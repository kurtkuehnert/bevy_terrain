use bevy::{
    asset::{ChangeWatcher, LoadState},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::*,
};
use bevy_terrain::prelude::*;
use std::time::Duration;

// const TILE_SIZE: u32 = 4000;
// const TILE_FORMAT: FileFormat = FileFormat::PNG;
// const RADIUS: f32 = 50.0;
// const TEXTURE_SIZE: u32 = 128;
// const MIP_LEVEL_COUNT: u32 = 1;
// const LOD_COUNT: u32 = 6;
// const HEIGHT: f32 = 200.0;
// const NODE_ATLAS_SIZE: u32 = 2048;
// const PATH: &str = "earth_4k";

const TILE_SIZE: u32 = 30000;
const TILE_FORMAT: FileFormat = FileFormat::TIF;
const RADIUS: f32 = 50.0;
const TEXTURE_SIZE: u32 = 512;
const MIP_LEVEL_COUNT: u32 = 3;
const LOD_COUNT: u32 = 8;
const HEIGHT: f32 = 4.0 / RADIUS;
const NODE_ATLAS_SIZE: u32 = 2048;
const PATH: &str = "earth_30k";

#[derive(AsBindGroup, TypeUuid, TypePath, Clone)]
#[uuid = "003e1d5d-241c-45a6-8c25-731dee22d820"]
pub struct TerrainMaterial {
    #[texture(0, dimension = "1d")]
    #[sampler(1)]
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

    // Use   magick mogrify -resize 1000x1000 -quality 100 -path ../earth_1k -format png *.tif
    // bevy_terrain::preprocess::cube_map::create_cube_map();

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
    let gradient = asset_server.load("textures/gradient.png");

    commands.insert_resource(LoadingTextures {
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

    // Preprocesses the terrain data.
    // Todo: Should be commented out after the first run.
    // loader.preprocess();

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = plugin_config.configure_terrain(
        TILE_SIZE as f32,
        LOD_COUNT,
        HEIGHT,
        NODE_ATLAS_SIZE,
        PATH.to_string(),
    );

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig {
        grid_size: 16,
        quadtree_size: 8,
        load_distance: 3.0,  // measured in nodes
        morph_distance: 8.0, // measured in tiles
        blend_distance: 1.5, // measured in nodes
        ..default()
    };

    // Create the terrain.
    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone(), Vec3::new(20.0, 30.0, -100.0), RADIUS),
            loader,
            materials.add(TerrainMaterial {
                gradient: gradient.clone(),
            }),
        ))
        .id();

    // Create the view.
    let view = commands
        .spawn((
            TerrainView,
            DebugCamera::default(),
            Camera3dBundle {
                projection: Projection::Perspective(PerspectiveProjection {
                    near: 0.001,
                    ..default()
                }),
                ..default()
            },
        ))
        .id();

    // Store the quadtree and the view config for the terrain and view.
    // This will hopefully be way nicer once the ECS can handle relations.
    let quadtree = Quadtree::from_configs(&config, &view_config);
    view_configs.insert((terrain, view), view_config.clone());
    quadtrees.insert((terrain, view), quadtree);

    // Create a sunlight for the physical based lighting.
    let light_direction = Vec3::new(-1.0, 0.0, 0.0);
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 20000.0,
            ..default()
        },
        transform: Transform::from_translation(light_direction).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::UVSphere {
            radius: 50.0,
            sectors: 50,
            stacks: 10,
        })),
        transform: Transform::from_translation(light_direction * 1000.0),
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
    gradient: Handle<Image>,
}

fn create_array_texture(
    asset_server: Res<AssetServer>,
    mut loading_textures: ResMut<LoadingTextures>,
    mut images: ResMut<Assets<Image>>,
) {
    if loading_textures.is_loaded
        || asset_server.get_load_state(loading_textures.gradient.clone()) != LoadState::Loaded
    {
        return;
    }

    loading_textures.is_loaded = true;

    let image = images.get_mut(&loading_textures.gradient).unwrap();
    image.texture_descriptor.dimension = TextureDimension::D1;
}
