use bevy::core_pipeline::core_3d::Camera3dDepthLoadOp;
use bevy::{math::DVec3, prelude::*, reflect::TypePath, render::render_resource::*};
use bevy_terrain::debug::OrbitalCameraController;
use bevy_terrain::picking::{PickingData, TerrainPickingPlugin};
use bevy_terrain::prelude::*;
use bevy_terrain::render::TerrainMaterial;

const PATH: &str = "/Volumes/ExternalSSD/tiles";
const RADIUS: f64 = 6371000.0;
const MAJOR_AXES: f64 = 6371000.0;
const MINOR_AXES: f64 = 6371000.0;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 16;

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct CustomMaterial {
    #[texture(0, dimension = "1d")]
    #[sampler(1)]
    gradient: Handle<Image>,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/spherical.wgsl".into()
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<TransformPlugin>(),
            TerrainPlugin,
            TerrainMaterialPlugin::<CustomMaterial>::default(),
            TerrainDebugPlugin, // enable debug settings and controls
            TerrainPickingPlugin,
        ))
        // .insert_resource(Msaa::Off)
        // .insert_resource(ClearColor(Color::WHITE))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<LoadingImages>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
    mut picking_data: ResMut<TerrainViewComponents<PickingData>>,
    asset_server: Res<AssetServer>,
) {
    let gradient = asset_server.load("textures/gradient.png");
    images.load_image(
        &gradient,
        TextureDimension::D1,
        TextureFormat::Rgba8UnormSrgb,
    );

    // Configure all the important properties of the terrain, as well as its attachments.
    let local_config = TerrainConfig {
        lod_count: LOD_COUNT,
        model: TerrainModel::ellipsoid(DVec3::ZERO, MAJOR_AXES, MINOR_AXES),
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
        mip_level_count: 1,
        format: AttachmentFormat::RF32,
    })
    .add_attachment(AttachmentConfig {
        name: "albedo".to_string(),
        texture_size: TEXTURE_SIZE,
        border_size: 1,
        mip_level_count: 1,
        format: AttachmentFormat::RgbU8,
    });

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let local_view_config = TerrainViewConfig::default();

    let local_tile_atlas = TileAtlas::new(&local_config);
    let local_tile_tree = TileTree::new(&local_tile_atlas, &local_view_config);

    // Configure all the important properties of the terrain, as well as its attachments.
    let global_config = TerrainConfig {
        lod_count: LOD_COUNT,
        model: TerrainModel::ellipsoid(DVec3::ZERO, MAJOR_AXES, MINOR_AXES),
        // model: TerrainModel::ellipsoid(
        //     DVec3::ZERO,
        //     6378137.0,
        //     6378137.0 * 0.5,
        //     MIN_HEIGHT,
        //     MAX_HEIGHT,
        // ),
        // model: TerrainModel::sphere(DVec3::ZERO, RADIUS),
        path: "/Volumes/ExternalSSD/tiles/earth".to_string(),
        ..default()
    }
    .add_attachment(AttachmentConfig {
        name: "height".to_string(),
        texture_size: TEXTURE_SIZE,
        border_size: 2,
        mip_level_count: 1,
        format: AttachmentFormat::RF32,
    });

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let global_view_config = TerrainViewConfig::default();

    let global_tile_atlas = TileAtlas::new(&global_config);
    let global_tile_tree = TileTree::new(&global_tile_atlas, &global_view_config);

    let (mut global_terrain, mut local_terrain, mut camera) = (
        Entity::PLACEHOLDER,
        Entity::PLACEHOLDER,
        Entity::PLACEHOLDER,
    );

    // commands.spawn_big_space(ReferenceFrame::new(10000000000000.0, 0.5), |root| {
    commands.spawn_big_space(ReferenceFrame::default(), |root| {
        let frame = root.frame().clone();

        global_terrain = root
            .spawn_spatial((
                setup_terrain(global_tile_atlas, &frame),
                TerrainMaterial(materials.add(CustomMaterial {
                    gradient: gradient.clone(),
                })),
            ))
            .id();

        local_terrain = root
            .spawn_spatial((
                setup_terrain(local_tile_atlas, &frame),
                TerrainMaterial(materials.add(CustomMaterial {
                    gradient: gradient.clone(),
                })),
            ))
            .id();

        camera = root
            .spawn_spatial((DebugCameraBundle::new(
                -DVec3::X * RADIUS * 3.0,
                RADIUS,
                &frame,
            ),))
            .id();
    });

    let global_view = commands
        .spawn((
            Camera {
                order: 0,
                ..default()
            },
            Camera3d {
                depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::TEXTURE_BINDING)
                    .into(),
                ..default()
            },
        ))
        .id();

    let local_view = commands
        .spawn((
            Camera {
                order: 1,
                ..default()
            },
            Camera3d {
                depth_load_op: Camera3dDepthLoadOp::Load,
                depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::TEXTURE_BINDING)
                    .into(),
                ..default()
            },
        ))
        .id();

    commands
        .entity(camera)
        .insert(OrbitalCameraController::new([
            (global_terrain, global_view),
            (local_terrain, local_view),
        ]))
        .add_children(&[local_view, global_view]);

    tile_trees.insert((global_terrain, global_view), global_tile_tree);
    tile_trees.insert((local_terrain, local_view), local_tile_tree);
    picking_data.insert((global_terrain, global_view), PickingData::default());
    picking_data.insert((local_terrain, local_view), PickingData::default());
}
