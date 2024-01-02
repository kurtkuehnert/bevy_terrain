use bevy::{
    asset::LoadState,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::*,
};
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/planar";
const TERRAIN_SIZE: f32 = 1000.0;
const HEIGHT: f32 = 500.0 / TERRAIN_SIZE;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 4;

#[derive(Asset, AsBindGroup, TypeUuid, TypePath, Clone)]
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
    App::new()
        .add_plugins((
            DefaultPlugins,
            TerrainPlugin,
            TerrainDebugPlugin, // enable debug settings and controls
            TerrainMaterialPlugin::<TerrainMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, create_array_texture)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    let gradient = asset_server.load("textures/gradient.png");
    commands.insert_resource(LoadingTexture {
        is_loaded: false,
        gradient: gradient.clone(),
    });

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        max_height: HEIGHT,
        path: PATH.to_string(),
        ..default()
    }
    .add_attachment(AttachmentConfig::new(
        "height".to_string(),
        TEXTURE_SIZE,
        2,
        AttachmentFormat::R16,
    ))
    .add_attachment(AttachmentConfig::new(
        "albedo".to_string(),
        TEXTURE_SIZE,
        1,
        AttachmentFormat::Rgba8,
    ));

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
            TerrainBundle::new(config.clone(), default(), TERRAIN_SIZE),
            materials.add(TerrainMaterial { gradient }),
        ))
        .id();

    // Create the view.
    let view = commands.spawn((TerrainView, DebugCamera::default())).id();

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
        || asset_server.load_state(loading_texture.gradient.clone()) != LoadState::Loaded
    {
        return;
    }

    loading_texture.is_loaded = true;

    let image = images.get_mut(&loading_texture.gradient).unwrap();
    image.texture_descriptor.dimension = TextureDimension::D1;
}
