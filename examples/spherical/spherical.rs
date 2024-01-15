use bevy::render::texture::ImageLoaderSettings;
use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::*,
};
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/spherical";
const RADIUS: f32 = 50.0;
const MIN_HEIGHT: f32 = -12.0 / 6371.0;
const MAX_HEIGHT: f32 = 9.0 / 6371.0;
const SUPER_ELEVATION: f32 = 10.0;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 5;

#[derive(Asset, AsBindGroup, TypeUuid, TypePath, Clone)]
#[uuid = "003e1d5d-241c-45a6-8c25-731dee22d820"]
pub struct TerrainMaterial {
    #[texture(0, dimension = "1d")]
    gradient: Handle<Image>,
    #[texture(1, dimension = "1d")]
    #[sampler(2)]
    gradient2: Handle<Image>,
    #[uniform(3)]
    index: u32,
    #[uniform(4)]
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
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    let gradient = asset_server.load_with_settings(
        "textures/gradient.png",
        |settings: &mut ImageLoaderSettings| settings.dimension = Some(TextureDimension::D1),
    );
    let gradient2 = asset_server.load_with_settings(
        "textures/gradient2.png",
        |settings: &mut ImageLoaderSettings| settings.dimension = Some(TextureDimension::D1),
    );

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        min_height: MIN_HEIGHT * SUPER_ELEVATION,
        max_height: MAX_HEIGHT * SUPER_ELEVATION,
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
        "height2".to_string(),
        TEXTURE_SIZE,
        2,
        AttachmentFormat::R16,
    ));

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig {
        grid_size: 32,
        quadtree_size: 8,
        load_distance: 3.0,
        morph_distance: 8.0,
        blend_distance: 1.5,
        ..default()
    };

    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone(), Vec3::new(20.0, 30.0, -100.0), RADIUS),
            materials.add(TerrainMaterial {
                gradient: gradient.clone(),
                gradient2: gradient2.clone(),
                index: 0,
                super_elevation: SUPER_ELEVATION,
            }),
        ))
        .id();

    let view = commands.spawn((TerrainView, DebugCamera::default())).id();

    initialize_terrain_view(
        terrain,
        view,
        &config,
        view_config,
        &mut quadtrees,
        &mut view_configs,
    );

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::UVSphere {
            radius: 50.0,
            sectors: 50,
            stacks: 10,
        })),
        transform: Transform::from_xyz(1000.0, 1000.0, 0.0),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        ..default()
    });
}
