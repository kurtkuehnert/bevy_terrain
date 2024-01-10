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
        .add_systems(Update, create_array_texture)
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
    let gradient = asset_server.load("textures/gradient.png");
    let gradient2 = asset_server.load("textures/gradient2.png");

    commands.insert_resource(LoadingTextures {
        textures: vec![gradient.clone(), gradient2.clone()],
    });

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        min_height: MIN_HEIGHT,
        max_height: MAX_HEIGHT,
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
        load_distance: 3.0,  // measured in nodes
        morph_distance: 8.0, // measured in tiles
        blend_distance: 1.5, // measured in nodes
        ..default()
    };

    // Create the terrain.
    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone(), Vec3::new(20.0, 30.0, -100.0), RADIUS),
            materials.add(TerrainMaterial {
                gradient: gradient.clone(),
                gradient2: gradient2.clone(),
                index: 0,
            }),
        ))
        .id();

    // Create the view.
    let view = commands.spawn((TerrainView, DebugCamera::default())).id();

    // Store the quadtree and the view config for the terrain and view.
    // This will hopefully be way nicer once the ECS can handle relations.
    let quadtree = Quadtree::from_configs(&config, &view_config);
    view_configs.insert((terrain, view), view_config.clone());
    quadtrees.insert((terrain, view), quadtree);

    // {
    //     // Configure all the important properties of the terrain, as well as its attachments.
    //     let config = TerrainConfig {
    //         lod_count: LOD_COUNT,
    //         min_height: MIN_HEIGHT,
    //         max_height: MAX_HEIGHT,
    //         path: PATH.to_string(),
    //         ..default()
    //     }
    //     .add_attachment(AttachmentConfig::new(
    //         "height2".to_string(),
    //         TEXTURE_SIZE,
    //         2,
    //         AttachmentFormat::R16,
    //     ));
    //
    //     // Create the terrain.
    //     let terrain = commands
    //         .spawn((
    //             TerrainBundle::new(
    //                 config.clone(),
    //                 Vec3::new(20.0, 30.0, -100.0),
    //                 RADIUS + 0.001,
    //             ),
    //             materials.add(TerrainMaterial {
    //                 gradient: gradient.clone(),
    //                 index: 1,
    //             }),
    //         ))
    //         .id();
    //
    //     // Store the quadtree and the view config for the terrain and view.
    //     // This will hopefully be way nicer once the ECS can handle relations.
    //     let quadtree = Quadtree::from_configs(&config, &view_config);
    //     view_configs.insert((terrain, view), view_config.clone());
    //     quadtrees.insert((terrain, view), quadtree);
    // }

    // Create a sunlight for the physical based lighting.
    let light_direction = Vec3::new(-1.0, 0.0, 0.0);
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 20000.0,
            ..default()
        },
        transform: Transform::from_translation(light_direction).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::UVSphere {
            radius: 50.0,
            sectors: 50,
            stacks: 10,
        })),
        transform: Transform::from_translation(light_direction * 1000.0),
        ..default()
    });
    commands.insert_resource(AmbientLight {
        brightness: 0.2,
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 5.0 })),
        ..default()
    });
}

#[derive(Resource)]
struct LoadingTextures {
    textures: Vec<Handle<Image>>,
}

fn create_array_texture(
    asset_server: Res<AssetServer>,
    mut loading_textures: ResMut<LoadingTextures>,
    mut images: ResMut<Assets<Image>>,
) {
    loading_textures.textures.retain(|handle| {
        if asset_server.is_loaded_with_dependencies(handle) {
            let image = images.get_mut(handle).unwrap();
            image.texture_descriptor.dimension = TextureDimension::D1;
            false
        } else {
            true
        }
    });
}
