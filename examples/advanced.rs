use bevy::{
    asset::LoadState,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{render_resource::*, texture::ImageSampler},
};
use bevy_terrain::prelude::*;

const TERRAIN_SIZE: u32 = 1024;
const TEXTURE_SIZE: u32 = 512;
const MIP_LEVEL_COUNT: u32 = 1;
const LOD_COUNT: u32 = 4;
const HEIGHT: f32 = 200.0;
const NODE_ATLAS_SIZE: u32 = 100;
const PATH: &str = "terrain";

#[derive(AsBindGroup, Asset, TypeUuid, TypePath, Clone)]
#[uuid = "4ccc53dd-2cfd-48ba-b659-c0e1a9bc0bdb"]
pub struct TerrainMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    array_texture: Handle<Image>,
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
        .add_plugins((
            DefaultPlugins,
            TerrainPlugin { config },
            TerrainDebugPlugin,
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
    let texture = asset_server.load("textures/array_texture.png");
    commands.insert_resource(LoadingTexture {
        is_loaded: false,
        handle: texture.clone(),
    });

    let mut loader = AttachmentFromDiskLoader::new(LOD_COUNT, PATH.to_string());
    loader.add_base_attachment(
        &plugin_config,
        TileConfig {
            path: "assets/terrain/source/height".to_string(),
            size: TERRAIN_SIZE,
            file_format: FileFormat::PNG,
        },
    );
    loader.add_attachment(
        &plugin_config,
        TileConfig {
            path: "assets/terrain/source/albedo.png".to_string(),
            size: TERRAIN_SIZE,
            file_format: FileFormat::PNG,
        },
        2,
    );

    // Preprocesses the terrain data.
    // Todo: Should be commented out after the first run.
    // loader.preprocess();

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = plugin_config.configure_terrain(
        TERRAIN_SIZE,
        LOD_COUNT,
        HEIGHT,
        NODE_ATLAS_SIZE,
        PATH.to_string(),
    );

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig {
        tile_scale: 4.0,
        grid_size: 4,
        node_count: 10,
        load_distance: 5.0,
        view_distance: 4.0,
        ..default()
    };

    // Create the terrain.
    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone()),
            loader,
            materials.add(TerrainMaterial {
                array_texture: texture,
            }),
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
    handle: Handle<Image>,
}

fn create_array_texture(
    asset_server: Res<AssetServer>,
    mut loading_texture: ResMut<LoadingTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    if loading_texture.is_loaded
        || asset_server.load_state(&loading_texture.handle) != LoadState::Loaded
    {
        return;
    }

    loading_texture.is_loaded = true;
    let image = images.get_mut(&loading_texture.handle).unwrap();
    image.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
        label: None,
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        address_mode_w: AddressMode::Repeat,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        ..default()
    });

    // Create a new array texture asset from the loaded texture.
    let array_layers = 4;
    image.reinterpret_stacked_2d_as_array(array_layers);
}
