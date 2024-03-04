use bevy::{prelude::*, reflect::TypePath, render::render_resource::*};
use bevy_terrain::prelude::*;

const RADIUS: f32 = 50.0;
const MIN_HEIGHT: f32 = -12.0 / 6371.0;
const MAX_HEIGHT: f32 = 9.0 / 6371.0;
const SUPER_ELEVATION: f32 = 10.0;
const TEXTURE_SIZE: u32 = 512;

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct TerrainMaterial {
    #[texture(0, dimension = "1d")]
    #[sampler(1)]
    gradient: Handle<Image>,
    #[uniform(2)]
    super_elevation: f32,
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
    App::new()
        .add_plugins((
            DefaultPlugins,
            TerrainPlugin,
            TerrainDebugPlugin, // enable debug settings and controls
            TerrainMaterialPlugin::<TerrainMaterial>::default(),
        ))
        .insert_resource(ClearColor(Color::WHITE))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<LoadingImages>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
    asset_server: Res<AssetServer>,
) {
    let gradient1 = asset_server.load("textures/gradient1.png");
    images.load_image(
        &gradient1,
        TextureDimension::D1,
        TextureFormat::Rgba8UnormSrgb,
    );

    let gradient2 = asset_server.load("textures/gradient2.png");
    images.load_image(
        &gradient2,
        TextureDimension::D1,
        TextureFormat::Rgba8UnormSrgb,
    );

    // Configure all the important properties of the terrain, as well as its attachments.
    let config1 = TerrainConfig {
        lod_count: 5,
        min_height: MIN_HEIGHT * SUPER_ELEVATION,
        max_height: MAX_HEIGHT * SUPER_ELEVATION,
        path: "terrains/spherical".to_string(),
        ..default()
    }
        .add_attachment(AttachmentConfig {
            name: "height".to_string(),
            texture_size: TEXTURE_SIZE,
            border_size: 2,
            mip_level_count: 4,
            format: AttachmentFormat::R16,
        });

    // let terrain1 = commands
    //     .spawn((
    //         TerrainBundle::new(config1.clone(), default(), RADIUS),
    //         materials.add(TerrainMaterial {
    //             gradient: gradient1,
    //             super_elevation: SUPER_ELEVATION,
    //         }),
    //     ))
    //     .id();

    // Configure all the important properties of the terrain, as well as its attachments.
    let config2 = TerrainConfig {
        lod_count: 8,
        min_height: MIN_HEIGHT * SUPER_ELEVATION,
        max_height: MAX_HEIGHT * SUPER_ELEVATION,
        path: "terrains/test".to_string(),
        ..default()
    }
        .add_attachment(AttachmentConfig {
            name: "height".to_string(),
            texture_size: TEXTURE_SIZE,
            border_size: 2,
            mip_level_count: 4,
            format: AttachmentFormat::R16,
        });

    let terrain2 = commands
        .spawn((
            TerrainBundle::new(config2.clone(), default(), RADIUS),
            materials.add(TerrainMaterial {
                gradient: gradient2,
                super_elevation: SUPER_ELEVATION,
            }),
        ))
        .id();

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig::default();
    let view = commands.spawn((TerrainView, DebugCamera::default())).id();

    // initialize_terrain_view(
    //     terrain1,
    //     view,
    //     &config1,
    //     view_config.clone(),
    //     &mut quadtrees,
    //     &mut view_configs,
    // );

    initialize_terrain_view(
        terrain2,
        view,
        &config2,
        view_config,
        &mut quadtrees,
        &mut view_configs,
    );

    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::new(100.0).mesh().build()),
        transform: Transform::from_xyz(-1000.0, 1000.0, -1000.0),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::default()),
        ..default()
    });
}
