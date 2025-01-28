use bevy::{
    math::DVec3,
    prelude::*,
    reflect::TypePath,
    render::{render_resource::*, storage::ShaderStorageBuffer},
    utils::HashSet,
};
use bevy_terrain::prelude::*;

const RADIUS: f64 = 6371000.0;

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
            TerrainPlugin::new(vec!["albedo"]),
            TerrainMaterialPlugin::<CustomMaterial>::default(),
            TerrainDebugPlugin, // enable debug settings and controls
            TerrainPickingPlugin,
        ))
        .init_resource::<TerrainConfigs>()
        .add_systems(Startup, setup_terrain_configs)
        .add_systems(Update, (load_terrain_config, spawn_terrains))
        .run();
}

#[derive(Resource, Default)]
struct TerrainConfigs {
    handles: HashSet<Handle<TerrainConfig>>,
    to_load: u32,
}

impl TerrainConfigs {
    fn add(&mut self, handle: Handle<TerrainConfig>) {
        self.handles.insert(handle);
        self.to_load += 1;
    }

    fn all_loaded(&self) -> bool {
        self.to_load == 0
    }
}

fn setup_terrain_configs(
    asset_server: Res<AssetServer>,
    mut terrain_configs: ResMut<TerrainConfigs>,
) {
    terrain_configs.add(asset_server.load("/Volumes/ExternalSSD/tiles/earth/config.tc.ron"));
    // Todo: loop over directory with datasets
}

fn load_terrain_config(
    mut handles: ResMut<TerrainConfigs>,
    mut events: EventReader<AssetEvent<TerrainConfig>>,
) {
    for &event in events.read() {
        if let AssetEvent::Added { .. } = event {
            handles.to_load -= 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_terrains(
    mut commands: Commands,
    mut images: ResMut<LoadingImages>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut tile_trees: ResMut<TerrainViewComponents<TileTree>>,
    asset_server: Res<AssetServer>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut handles: ResMut<TerrainConfigs>,
    mut terrain_configs: ResMut<Assets<TerrainConfig>>,
    terrain_attachments: Res<TerrainAttachments>,
) {
    if !handles.all_loaded() {
        return;
    }

    handles.to_load = 1; // only spawn once

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

    let config = terrain_configs
        .get_mut(
            asset_server
                .load("/Volumes/ExternalSSD/tiles/earth/config.tc.ron")
                .id(),
        )
        .unwrap();

    let view_config = TerrainViewConfig {
        order: 1,
        tree_size: 16,
        blend_distance: 8.0,
        ..default()
    };

    let material = CustomMaterial {
        gradient: gradient.clone(),
        gradient_info: GradientInfo {
            min: -12000.0,
            max: 9000.0,
            custom: 1,
        },
    };

    let (mut terrain, mut view) = (Entity::PLACEHOLDER, Entity::PLACEHOLDER);

    commands.spawn_big_space(Grid::default(), |root| {
        let grid = root.grid().clone();

        let (cell, translation) = grid.translation_to_grid(-DVec3::X * RADIUS * 3.0);

        view = root
            .spawn_spatial((
                DebugCameraController::new(RADIUS),
                OrbitalCameraController::default(),
                Transform::from_translation(translation).looking_to(Vec3::X, Vec3::Y),
                cell,
            ))
            .id();

        terrain = root
            .spawn_spatial((
                TileAtlas::new(config, &mut buffers, &terrain_attachments),
                TerrainMaterial(materials.add(material)),
            ))
            .id();

        tile_trees.insert(
            (terrain, view),
            TileTree::new(
                config,
                &view_config,
                (terrain, view),
                root.commands(),
                &mut buffers,
            ),
        );
    });
}
