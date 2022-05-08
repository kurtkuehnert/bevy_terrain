use crate::{
    config::NodeId,
    node_atlas::{LoadNodeEvent, NodeAtlas},
    TerrainConfig,
};
use bevy::{
    asset::{AssetServer, HandleId, LoadState},
    prelude::*,
    render::render_resource::*,
    utils::HashMap,
};

#[derive(Clone)]
pub enum NodeAttachmentData {
    Buffer { data: Vec<u8> },
    Texture { handle: Handle<Image> },
}

pub enum NodeAttachment {
    Buffer {
        binding: u32,
        buffer: Buffer,
    },
    Texture {
        view_binding: u32,
        sampler_binding: u32,
        texture_size: u32,
        texture: Texture,
        view: TextureView,
        sampler: Sampler,
    },
}

#[derive(Clone)]
pub enum NodeAttachmentConfig {
    Buffer {
        binding: u32,
        descriptor: BufferDescriptor<'static>,
    },
    Texture {
        view_binding: u32,
        sampler_binding: u32,
        texture_size: u32,
        texture_descriptor: TextureDescriptor<'static>,
        view_descriptor: TextureViewDescriptor<'static>,
        sampler_descriptor: SamplerDescriptor<'static>,
    },
}

pub fn add_height_attachment_config(config: &mut TerrainConfig, texture_size: u32) {
    let texture_descriptor = TextureDescriptor {
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

    let view_descriptor = TextureViewDescriptor {
        label: None,
        format: None,
        dimension: Some(TextureViewDimension::D2Array),
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    };

    let sampler_descriptor = SamplerDescriptor {
        label: None,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        compare: None,
        anisotropy_clamp: None,
        border_color: None,
    };

    config.add_node_attachment_config(
        "height_map".into(),
        NodeAttachmentConfig::Texture {
            view_binding: 2,
            sampler_binding: 3,
            texture_size,
            texture_descriptor,
            view_descriptor,
            sampler_descriptor,
        },
    );
}

pub fn add_albedo_attachment_config(config: &mut TerrainConfig, texture_size: u32) {
    let texture_descriptor = TextureDescriptor {
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

    let view_descriptor = TextureViewDescriptor {
        label: None,
        format: None,
        dimension: Some(TextureViewDimension::D2Array),
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    };

    let sampler_descriptor = SamplerDescriptor {
        label: None,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        compare: None,
        anisotropy_clamp: None,
        border_color: None,
    };

    config.add_node_attachment_config(
        "albedo_map".into(),
        NodeAttachmentConfig::Texture {
            view_binding: 4,
            sampler_binding: 5,
            texture_size,
            texture_descriptor,
            view_descriptor,
            sampler_descriptor,
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
                    node.loading_attachments.remove(label);
                } else {
                    handle_mapping.insert(handle.id, (node_id, label.clone()));
                };

                node.attachment_data
                    .insert(label.clone(), NodeAttachmentData::Texture { handle });
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
                    node.loading_attachments.remove(&label);
                    break;
                }
            }
        }
    }
}
