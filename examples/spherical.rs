use bevy::{
    math::DVec3, prelude::*, reflect::TypePath, render::render_resource::*,
    render::storage::ShaderStorageBuffer,
};
use bevy_terrain::prelude::*;

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
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<LoadingImages>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
    asset_server: Res<AssetServer>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let gradient = asset_server.load("textures/gradient.png");
    images.load_image(
        &gradient,
        TextureDimension::D1,
        TextureFormat::Rgba8UnormSrgb,
    );

    // Configure all the important properties of the terrain, as well as its attachments.
    // let local_config = TerrainConfig {
    //     lod_count: LOD_COUNT,
    //     model: TerrainModel::ellipsoid(DVec3::ZERO, MAJOR_AXES, MINOR_AXES),
    //     path: "/Volumes/ExternalSSD/tiles".to_string(),
    //     ..default()
    // }
    // .add_attachment(AttachmentConfig {
    //     name: "height".to_string(),
    //     texture_size: TEXTURE_SIZE,
    //     border_size: 2,
    //     mip_level_count: 1,
    //     format: AttachmentFormat::RF32,
    // })
    // .add_attachment(AttachmentConfig {
    //     name: "albedo".to_string(),
    //     texture_size: TEXTURE_SIZE,
    //     border_size: 1,
    //     mip_level_count: 1,
    //     format: AttachmentFormat::RgbU8,
    // });
    //
    // // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    // let local_view_config = TerrainViewConfig {
    //     order: 0,
    //     ..default()
    // };

    // Configure all the important properties of the terrain, as well as its attachments.
    let global_config = TerrainConfig {
        lod_count: LOD_COUNT,
        model: TerrainModel::ellipsoid(DVec3::ZERO, MAJOR_AXES, MINOR_AXES),
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
    let global_view_config = TerrainViewConfig {
        order: 1,
        ..default()
    };

    commands.spawn_big_space(ReferenceFrame::default(), |root| {
        let frame = root.frame().clone();

        let global_terrain = root
            .spawn_spatial((
                TileAtlas::new(&global_config),
                TerrainMaterial(materials.add(CustomMaterial {
                    gradient: gradient.clone(),
                })),
            ))
            .id();

        // let local_terrain = root
        //     .spawn_spatial((
        //         TileAtlas::new(&local_config),
        //         TerrainMaterial(materials.add(CustomMaterial {
        //             gradient: gradient.clone(),
        //         })),
        //     ))
        //     .id();

        let (cell, translation) = frame.translation_to_grid(-DVec3::X * RADIUS * 3.0);

        let view = root
            .spawn_spatial((
                DebugCameraController::new(RADIUS),
                OrbitalCameraController::default(),
                Transform::from_translation(translation).looking_to(Vec3::X, Vec3::Y),
                cell,
            ))
            .id();

        let tile_tree = TileTree::new(&global_config, &global_view_config, &mut buffers);

        tile_trees.insert((global_terrain, view), tile_tree);
        // tile_trees.insert(
        //     (local_terrain, view),
        //     TileTree::new(&local_config, &local_view_config),
        // );
    });
}
