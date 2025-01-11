use bevy::{
    math::DVec3, prelude::*, reflect::TypePath, render::render_resource::*,
    render::storage::ShaderStorageBuffer,
};
use bevy_terrain::prelude::*;

const RADIUS: f64 = 6371000.0;
const MAJOR_AXES: f64 = 6371000.0;
const MINOR_AXES: f64 = 6371000.0;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 15;

#[derive(ShaderType, Clone)]
struct GradientInfo {
    min: f32,
    max: f32,
    custom: u32,
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct CustomMaterial {
    #[texture(0, dimension = "1d")]
    #[sampler(1)]
    gradient: Handle<Image>,
    #[uniform(2)]
    gradient_info: GradientInfo,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/spherical.wgsl".into()
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                // .set(WindowPlugin {
                //     primary_window: Some(Window {
                //         mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                //         ..default()
                //     }),
                //     ..default()
                // })
                .build()
                .disable::<TransformPlugin>(),
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

    let gradient2 = asset_server.load("textures/gradient2.png");
    images.load_image(
        &gradient2,
        TextureDimension::D1,
        TextureFormat::Rgba8UnormSrgb,
    );

    // Configure all the important properties of the terrain, as well as its attachments.
    let local_config = TerrainConfig {
        lod_count: LOD_COUNT,
        model: TerrainModel::ellipsoid(DVec3::ZERO, MAJOR_AXES, MINOR_AXES),
        path: "/Volumes/ExternalSSD/tiles/local".to_string(),
        atlas_size: 2048,
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
    let local_view_config = TerrainViewConfig {
        order: 0,
        tree_size: 16,
        blend_distance: 8.0,
        ..default()
    };

    // Configure all the important properties of the terrain, as well as its attachments.
    let global_config = TerrainConfig {
        lod_count: 8,
        model: TerrainModel::ellipsoid(DVec3::ZERO, MAJOR_AXES, MINOR_AXES),
        path: "/Volumes/ExternalSSD/tiles/earth".to_string(),
        atlas_size: 2048,
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
        tree_size: 16,
        blend_distance: 8.0,
        ..default()
    };

    let scope_config = TerrainConfig {
        lod_count: LOD_COUNT,
        model: TerrainModel::ellipsoid(DVec3::ZERO, MAJOR_AXES, MINOR_AXES),
        path: "/Volumes/ExternalSSD/tiles/scope".to_string(),
        atlas_size: 2048,
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
    let scope_view_config = TerrainViewConfig {
        order: 0,
        tree_size: 12,
        blend_distance: 8.0,
        ..default()
    };

    let (mut global_terrain, mut local_terrain, mut view) = (
        Entity::PLACEHOLDER,
        Entity::PLACEHOLDER,
        Entity::PLACEHOLDER,
    );

    let mut scope_terrain = Entity::PLACEHOLDER;

    commands.spawn_big_space(ReferenceFrame::default(), |root| {
        let frame = root.frame().clone();

        global_terrain = root
            .spawn_spatial((
                TileAtlas::new(&global_config, &mut buffers),
                TerrainMaterial(materials.add(CustomMaterial {
                    gradient: gradient.clone(),
                    gradient_info: GradientInfo {
                        min: -12000.0,
                        max: 9000.0,
                        custom: 1,
                    },
                })),
            ))
            .id();

        local_terrain = root
            .spawn_spatial((
                TileAtlas::new(&local_config, &mut buffers),
                TerrainMaterial(materials.add(CustomMaterial {
                    gradient: gradient.clone(),
                    gradient_info: GradientInfo {
                        min: -12000.0,
                        max: 9000.0,
                        custom: 1,
                    },
                })),
            ))
            .id();

        scope_terrain = root
            .spawn_spatial((
                TileAtlas::new(&scope_config, &mut buffers),
                TerrainMaterial(materials.add(CustomMaterial {
                    gradient: gradient2.clone(),
                    gradient_info: GradientInfo {
                        min: -3806.439,
                        max: -197.742,
                        custom: 0,
                    },
                })),
            ))
            .id();

        let (cell, translation) = frame.translation_to_grid(-DVec3::X * RADIUS * 3.0);

        view = root
            .spawn_spatial((
                DebugCameraController::new(RADIUS),
                OrbitalCameraController::default(),
                Transform::from_translation(translation).looking_to(Vec3::X, Vec3::Y),
                cell,
            ))
            .id();
    });

    tile_trees.insert(
        (global_terrain, view),
        TileTree::new(
            &global_config,
            &global_view_config,
            (global_terrain, view),
            &mut commands,
            &mut buffers,
        ),
    );

    tile_trees.insert(
        (local_terrain, view),
        TileTree::new(
            &local_config,
            &local_view_config,
            (local_terrain, view),
            &mut commands,
            &mut buffers,
        ),
    );

    tile_trees.insert(
        (scope_terrain, view),
        TileTree::new(
            &scope_config,
            &scope_view_config,
            (scope_terrain, view),
            &mut commands,
            &mut buffers,
        ),
    );
}
