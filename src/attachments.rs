use crate::{
    config::NodeId,
    node_atlas::{LoadNodeEvent, NodeAtlas, NodeAttachment},
    render::gpu_node_atlas::AtlasAttachmentConfig,
    TerrainConfig,
};
use bevy::{
    asset::{AssetServer, HandleId, LoadState},
    prelude::*,
    render::render_resource::*,
    utils::HashMap,
};

pub fn add_sampler_attachment_config(config: &mut TerrainConfig) {
    let sampler_descriptor = SamplerDescriptor {
        label: "sampler_attachment".into(),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    };

    config.add_node_attachment_config(
        "sampler".into(),
        AtlasAttachmentConfig::Sampler {
            binding: 2,
            sampler_descriptor,
        },
    );
}

pub fn add_height_attachment_config(config: &mut TerrainConfig, texture_size: u32) {
    let texture_descriptor = TextureDescriptor {
        label: "height_attachment_texture".into(),
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
        label: "height_attachment_view".into(),
        dimension: Some(TextureViewDimension::D2Array),
        aspect: TextureAspect::All,
        ..default()
    };

    config.add_node_attachment_config(
        "height_map".into(),
        AtlasAttachmentConfig::Texture {
            binding: 3,
            texture_size,
            texture_descriptor,
            view_descriptor,
        },
    );
}

pub fn add_albedo_attachment_config(config: &mut TerrainConfig, texture_size: u32) {
    let texture_descriptor = TextureDescriptor {
        label: "albedo_attachment_texture".into(),
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

    let view_descriptor = TextureViewDescriptor {
        label: "albedo_attachment_view".into(),
        dimension: Some(TextureViewDimension::D2Array),
        aspect: TextureAspect::All,
        ..default()
    };

    config.add_node_attachment_config(
        "albedo_map".into(),
        AtlasAttachmentConfig::Texture {
            binding: 4,
            texture_size,
            texture_descriptor,
            view_descriptor,
        },
    );
}

pub struct AttachmentFromDisk {
    pub path: String,
    pub texture_descriptor: TextureDescriptor<'static>,
}

#[derive(Component)]
pub struct AttachmentFromDiskConfig {
    pub attachments: HashMap<String, AttachmentFromDisk>,
    /// Maps the id of an asset to the corresponding node id.
    pub handle_mapping: HashMap<HandleId, (NodeId, String)>,
}

pub fn start_loading_attachment_from_disk(
    mut load_events: EventReader<LoadNodeEvent>,
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<(&mut NodeAtlas, &mut AttachmentFromDiskConfig)>,
) {
    for (mut node_atlas, mut config) in terrain_query.iter_mut() {
        let AttachmentFromDiskConfig {
            ref mut attachments,
            ref mut handle_mapping,
        } = config.as_mut();

        for &LoadNodeEvent(node_id) in load_events.iter() {
            let node = node_atlas.loading_nodes.get_mut(&node_id).unwrap();

            for (label, AttachmentFromDisk { ref path, .. }) in attachments.iter() {
                let handle: Handle<Image> = asset_server.load(&format!("{path}/{node_id}.png"));

                if asset_server.get_load_state(handle.clone()) == LoadState::Loaded {
                    node.loaded(label);
                } else {
                    handle_mapping.insert(handle.id, (node_id, label.clone()));
                };

                node.set_attachment(label.clone(), NodeAttachment::Texture { handle });
            }
        }
    }
}

pub fn finish_loading_attachment_from_disk(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut terrain_query: Query<(&mut NodeAtlas, &mut AttachmentFromDiskConfig)>,
) {
    for event in asset_events.iter() {
        if let AssetEvent::Created { handle } = event {
            for (mut node_atlas, mut config) in terrain_query.iter_mut() {
                if let Some((node_id, label)) = config.handle_mapping.remove(&handle.id) {
                    let image = images.get_mut(handle).unwrap();
                    let attachment = config.attachments.get(&label).unwrap();

                    image.texture_descriptor = attachment.texture_descriptor.clone();

                    let node = node_atlas.loading_nodes.get_mut(&node_id).unwrap();
                    node.loaded(&label);
                    break;
                }
            }
        }
    }
}
