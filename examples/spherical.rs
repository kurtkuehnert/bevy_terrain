use bevy::{math::DVec3, prelude::*, reflect::TypePath, render::render_resource::*};
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/spherical";
const RADIUS: f64 = 6371000.0;
const MIN_HEIGHT: f32 = -12000.0;
const MAX_HEIGHT: f32 = 9000.0;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 16;

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct TerrainMaterial {
    #[texture(0, dimension = "1d")]
    #[sampler(1)]
    gradient: Handle<Image>,
}

impl Material for TerrainMaterial {
    // fn vertex_shader() -> ShaderRef {
    //     "shaders/spherical.wgsl".into()
    // }
    fn fragment_shader() -> ShaderRef {
        "shaders/spherical.wgsl".into()
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<TransformPlugin>(),
            TerrainPlugin,
            TerrainMaterialPlugin::<TerrainMaterial>::default(),
            TerrainDebugPlugin, // enable debug settings and controls
        ))
        // .insert_resource(ClearColor(Color::WHITE))
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
    let gradient = asset_server.load("textures/gradient.png");
    images.load_image(
        &gradient,
        TextureDimension::D1,
        TextureFormat::Rgba8UnormSrgb,
    );

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        model: TerrainModel::ellipsoid(
            DVec3::ZERO,
            6378137.0,
            6356752.314245,
            MIN_HEIGHT,
            MAX_HEIGHT,
        ),
        // model: TerrainModel::ellipsoid(
        //     DVec3::ZERO,
        //     6378137.0,
        //     6378137.0 * 0.5,
        //     MIN_HEIGHT,
        //     MAX_HEIGHT,
        // ),
        // model: TerrainModel::sphere(DVec3::ZERO, RADIUS),
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

    commands.spawn_big_space(ReferenceFrame::default(), |root| {
        let frame = root.frame().clone();

        let terrain = root
            .spawn_spatial((
                TerrainBundle::new(config.clone(), &frame),
                materials.add(TerrainMaterial {
                    gradient: gradient.clone(),
                }),
            ))
            .id();

        let view = root
            .spawn_spatial((
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

        let sun_position = DVec3::new(-1.0, 1.0, -1.0) * RADIUS * 10.0;
        let (sun_cell, sun_translation) = frame.translation_to_grid(sun_position);

        root.spawn_spatial((
            PbrBundle {
                mesh: meshes.add(Sphere::new(RADIUS as f32 * 2.0).mesh().build()),
                transform: Transform::from_translation(sun_translation),
                ..default()
            },
            sun_cell,
        ));

        root.spawn_spatial(PbrBundle {
            mesh: meshes.add(Cuboid::from_length(RADIUS as f32 * 0.1)),
            ..default()
        });
    });
}
