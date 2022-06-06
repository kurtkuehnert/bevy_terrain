use bevy::{
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
    render::{camera::Camera3d, render_resource::*},
};
use bevy_terrain::{
    attachment::{AtlasAttachmentConfig, AttachmentIndex},
    attachment_loader::{TextureAttachmentFromDisk, TextureAttachmentFromDiskLoader},
    bundles::TerrainBundle,
    config::TerrainConfig,
    preprocess::new::{preprocess_tiles, ImageFormat},
    TerrainPlugin,
};

fn main() {
    let mut app = App::new();

    app.insert_resource(WindowDescriptor {
        title: "Terrain Rendering".into(),
        ..default()
    })
    .add_plugins(DefaultPlugins)
    .add_plugin(TerrainPlugin)
    .add_plugin(WireframePlugin)
    .add_startup_system(setup)
    .add_system(toggle_wireframe_system);

    app.run()
}

fn setup(mut commands: Commands) {
    let mut from_disk_loader = TextureAttachmentFromDiskLoader::default();

    let mut config =
        TerrainConfig::new(128, 5, UVec2::new(2, 2), 1.0, 200.0, "terrain/".to_string());

    setup_default_sampler(&mut config, 2);
    setup_height_texture(&mut config, &mut from_disk_loader, 3, 128 + 4);

    // Should only be run once. Comment out after the first run.
    preprocess_tiles(
        "assets/terrain/height",
        "assets/terrain/data/height",
        0,
        5,
        (0, 0),
        1024,
        128,
        2,
        ImageFormat::LUMA16,
    );

    commands
        .spawn_bundle(TerrainBundle::new(config))
        .insert(from_disk_loader)
        .insert(Wireframe);

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            camera: Camera::default(),
            perspective_projection: PerspectiveProjection {
                far: 10000.0,
                ..default()
            },
            transform: Transform::from_xyz(-200.0, 500.0, -200.0)
                .looking_at(Vec3::new(500.0, 0.0, 500.0), Vec3::Y),
            ..default()
        })
        .insert(Camera3d);
}

pub(crate) fn setup_default_sampler(config: &mut TerrainConfig, attachment_index: AttachmentIndex) {
    let sampler_descriptor = SamplerDescriptor {
        label: "default_sampler_attachment".into(),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        ..default()
    };

    config.add_attachment(
        attachment_index,
        AtlasAttachmentConfig::Sampler { sampler_descriptor },
    );
}

pub(crate) fn setup_height_texture(
    config: &mut TerrainConfig,
    from_disk_loader: &mut TextureAttachmentFromDiskLoader,
    attachment_index: AttachmentIndex,
    texture_size: u32,
) {
    let atlas_texture_descriptor = TextureDescriptor {
        label: "atlas_height_texture_attachment".into(),
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

    let view_descriptor = TextureViewDescriptor {
        label: "height_texture_attachment_view".into(),
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    };

    let mut node_texture_descriptor = atlas_texture_descriptor.clone();
    node_texture_descriptor.label = "node_height_texture_attachment".into();
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
            view_descriptor,
        },
    );
}

fn toggle_wireframe_system(
    mut commands: Commands,
    input: Res<Input<KeyCode>>,
    terrain_query: Query<(Entity, Option<&Wireframe>), With<TerrainConfig>>,
) {
    if input.just_pressed(KeyCode::W) {
        for (entity, wireframe) in terrain_query.iter() {
            match wireframe {
                None => commands.entity(entity).insert(Wireframe),
                Some(_) => commands.entity(entity).remove::<Wireframe>(),
            };
        }
    }
}
