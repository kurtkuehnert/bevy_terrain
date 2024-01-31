use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{render_resource::*, texture::ImageLoaderSettings},
};
use bevy_terrain::prelude::*;

const PATH: &str = "terrains/planar";
const TERRAIN_SIZE: f32 = 1000.0;
const HEIGHT: f32 = 500.0 / TERRAIN_SIZE;
const TEXTURE_SIZE: u32 = 512;
const LOD_COUNT: u32 = 4;

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct TerrainMaterial {
    #[texture(0, dimension = "1d")]
    #[sampler(1)]
    gradient: Handle<Image>,
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/advanced.wgsl".into()
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
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
    asset_server: Res<AssetServer>,
) {
    let gradient = asset_server.load_with_settings(
        "textures/gradient2.png",
        |settings: &mut ImageLoaderSettings| {
            settings.texture_dimension = Some(TextureDimension::D1)
        },
    );

    // Configure all the important properties of the terrain, as well as its attachments.
    let config = TerrainConfig {
        lod_count: LOD_COUNT,
        max_height: HEIGHT,
        path: PATH.to_string(),
        ..default()
    }
    .add_attachment(AttachmentConfig {
        name: "height".to_string(),
        texture_size: TEXTURE_SIZE,
        border_size: 2,
        mip_level_count: 4,
        format: AttachmentFormat::R16,
    })
    .add_attachment(AttachmentConfig {
        name: "albedo".to_string(),
        texture_size: TEXTURE_SIZE,
        border_size: 2,
        mip_level_count: 4,
        format: AttachmentFormat::Rgba8,
    });

    // Configure the quality settings of the terrain view. Adapt the settings to your liking.
    let view_config = TerrainViewConfig::default();

    let terrain = commands
        .spawn((
            TerrainBundle::new(config.clone(), default(), TERRAIN_SIZE),
            materials.add(TerrainMaterial { gradient }),
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
}
