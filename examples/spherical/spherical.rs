use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{render_resource::*, texture::ImageLoaderSettings},
};
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/spherical";
const RADIUS: f32 = 50.0;
const MIN_HEIGHT: f32 = -12.0 / 6371.0;
const MAX_HEIGHT: f32 = 9.0 / 6371.0;
const SUPER_ELEVATION: f32 = 10.0;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 5;

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
        |settings: &mut ImageLoaderSettings| {
            settings.texture_dimension = Some(TextureDimension::D1)
        },
    );

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        min_height: MIN_HEIGHT * SUPER_ELEVATION,
        max_height: MAX_HEIGHT * SUPER_ELEVATION,
        path: PATH.to_string(),
        ..default()
    }
    .add_attachment(AttachmentConfig {
        name: "height".to_string(),
        texture_size: TEXTURE_SIZE,
        border_size: 2,
        mip_level_count: 4,
        format: AttachmentFormat::R16,
    });

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig::default();

    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone(), Vec3::new(100.0, 100.0, 100.0), RADIUS),
            materials.add(TerrainMaterial {
                gradient: gradient.clone(),
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
            radius: 100.0,
            ..default()
        })),
        transform: Transform::from_xyz(-1000.0, 1000.0, -1000.0),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube::default())),
        ..default()
    });
}
