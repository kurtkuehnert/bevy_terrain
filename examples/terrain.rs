use bevy::{prelude::*, render::render_resource::*};
use bevy_terrain::{
    attachment::{AtlasAttachmentConfig, AttachmentIndex},
    attachment_loader::{TextureAttachmentFromDisk, TextureAttachmentFromDiskLoader},
    bundles::TerrainBundle,
    preprocess::{preprocess_tiles, ImageFormat},
    quadtree::Quadtree,
    terrain::TerrainConfig,
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
    TerrainPlugin,
};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugin(TerrainPlugin)
        .add_startup_system(setup);

    // Should only be run once. Comment out after the first run.
    preprocess_tiles(
        "assets/terrain/source/height",
        "assets/terrain/data/height",
        0,
        5,
        (0, 0),
        1024,
        128,
        2,
        ImageFormat::LUMA16,
    );

    // Should only be run once. Comment out after the first run.
    preprocess_tiles(
        "assets/terrain/source/albedo.png",
        "assets/terrain/data/albedo",
        0,
        5,
        (0, 0),
        2048,
        256,
        1,
        ImageFormat::RGB,
    );

    app.run()
}

fn setup(
    mut commands: Commands,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut terrain_view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
) {
    let mut from_disk_loader = TextureAttachmentFromDiskLoader::default();
    let mut config = TerrainConfig::new(128, 5, 200.0, "terrain/".to_string());

    setup_default_sampler(&mut config, 1);
    setup_height_texture(&mut config, &mut from_disk_loader, 2, 128 + 4);
    setup_albedo_texture(&mut config, &mut from_disk_loader, 3, 256 + 2);

    let terrain = commands
        .spawn_bundle(TerrainBundle::new(config.clone()))
        .insert(from_disk_loader)
        .id();

    let view = commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(-200.0, 500.0, -200.0)
                .looking_at(Vec3::new(500.0, 0.0, 500.0), Vec3::Y),
            ..default()
        })
        .insert(TerrainView)
        .id();

    let view_config = TerrainViewConfig::new(1024, 16, 3.0, 2.0, 0.5);
    let quadtree = Quadtree::new(&config, &view_config);

    terrain_view_configs.insert((terrain, view), view_config);
    quadtrees.insert((terrain, view), quadtree);

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(0.0, 200.0, 0.0),
        ..default()
    });
}

fn setup_default_sampler(config: &mut TerrainConfig, attachment_index: AttachmentIndex) {
    let sampler_descriptor = SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    };

    config.add_attachment(
        attachment_index,
        AtlasAttachmentConfig::Sampler { sampler_descriptor },
    );
}

fn setup_height_texture(
    config: &mut TerrainConfig,
    from_disk_loader: &mut TextureAttachmentFromDiskLoader,
    attachment_index: AttachmentIndex,
    texture_size: u32,
) {
    let atlas_texture_descriptor = TextureDescriptor {
        label: None,
        size: Extent3d {
            width: texture_size,
            height: texture_size,
            depth_or_array_layers: config.node_atlas_size as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R16Unorm,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
    };

    let mut node_texture_descriptor = atlas_texture_descriptor.clone();
    node_texture_descriptor.size.depth_or_array_layers = 1;
    node_texture_descriptor.usage |= TextureUsages::COPY_SRC;

    from_disk_loader.add_attachment(
        attachment_index,
        TextureAttachmentFromDisk {
            path: config.path.clone() + "data/height",
            texture_descriptor: node_texture_descriptor,
        },
    );

    config.add_attachment(
        attachment_index,
        AtlasAttachmentConfig::Texture {
            texture_size,
            texture_descriptor: atlas_texture_descriptor,
            view_descriptor: default(),
        },
    );
}

fn setup_albedo_texture(
    config: &mut TerrainConfig,
    from_disk_loader: &mut TextureAttachmentFromDiskLoader,
    attachment_index: AttachmentIndex,
    texture_size: u32,
) {
    let atlas_texture_descriptor = TextureDescriptor {
        label: None,
        size: Extent3d {
            width: texture_size,
            height: texture_size,
            depth_or_array_layers: config.node_atlas_size as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
    };

    let mut node_texture_descriptor = atlas_texture_descriptor.clone();
    node_texture_descriptor.size.depth_or_array_layers = 1;
    node_texture_descriptor.usage |= TextureUsages::COPY_SRC;

    from_disk_loader.add_attachment(
        attachment_index,
        TextureAttachmentFromDisk {
            path: config.path.clone() + "data/albedo",
            texture_descriptor: node_texture_descriptor,
        },
    );

    config.add_attachment(
        attachment_index,
        AtlasAttachmentConfig::Texture {
            texture_size,
            texture_descriptor: atlas_texture_descriptor,
            view_descriptor: default(),
        },
    );
}
