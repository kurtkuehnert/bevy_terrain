use bevy::math::DVec3;
use bevy::window::Cursor;
use bevy::{prelude::*, reflect::TypePath, render::render_resource::*};
use bevy_terrain::big_space::{FloatingOriginPlugin, GridCell, RootReferenceFrame};
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/spherical";
const RADIUS: f64 = 6371000.0;
const MIN_HEIGHT: f32 = -12.0 / 6371.0;
const MAX_HEIGHT: f32 = 9.0 / 6371.0;
const SUPER_ELEVATION: f32 = 10.0;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 16;

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
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        cursor: Cursor {
                            visible: false,
                            ..default()
                        },
                        ..default()
                    }),
                    ..default()
                })
                .build()
                .disable::<TransformPlugin>(),
            TerrainPlugin,
            TerrainDebugPlugin, // enable debug settings and controls
            TerrainMaterialPlugin::<TerrainMaterial>::default(),
            FloatingOriginPlugin::default(),
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
    frame: Res<RootReferenceFrame>,
) {
    let gradient = asset_server.load("textures/gradient.png");
    images.load_image(
        &gradient,
        TextureDimension::D1,
        TextureFormat::Rgba8UnormSrgb,
    );

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        scale: RADIUS,
        min_height: MIN_HEIGHT * SUPER_ELEVATION,
        max_height: MAX_HEIGHT * SUPER_ELEVATION,
        path: PATH.to_string(),
        ..default()
    };
    // .add_attachment(AttachmentConfig {
    //     name: "height".to_string(),
    //     texture_size: TEXTURE_SIZE,
    //     border_size: 2,
    //     mip_level_count: 4,
    //     format: AttachmentFormat::R16,
    // });

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig::default();

    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone(), DVec3::new(0.0, 0.0, 0.0), &frame),
            materials.add(TerrainMaterial {
                gradient: gradient.clone(),
                super_elevation: SUPER_ELEVATION,
            }),
        ))
        .id();

    let view = commands
        .spawn((
            TerrainView,
            DebugCameraBundle::new(-DVec3::X * RADIUS * 3.0, RADIUS, &frame),
        ))
        .id();

    initialize_terrain_view(
        terrain,
        view,
        &config,
        view_config,
        &mut quadtrees,
        &mut view_configs,
    );

    let sun_position = DVec3::new(-1000.0, 1000.0, -1000.0);
    let (sun_cell, sun_translation) = frame.translation_to_grid(sun_position);

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(100.0).mesh().build()),
            transform: Transform::from_translation(sun_translation),
            ..default()
        },
        sun_cell,
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            ..default()
        },
        GridCell::default(),
    ));
}
