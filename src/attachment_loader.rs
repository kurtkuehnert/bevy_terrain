//! The default attachment loader, which loads node data from disk.

use crate::terrain_data::{
    node_atlas::NodeAtlas, AttachmentConfig, AttachmentIndex, FileFormat, NodeId,
};
use bevy::{
    asset::{AssetServer, HandleId, LoadState},
    prelude::*,
    render::render_resource::*,
    utils::HashMap,
};

pub(crate) struct AttachmentFromDisk {
    pub(crate) path: String,
    pub(crate) format: TextureFormat,
    pub(crate) file_format: FileFormat,
}

impl AttachmentFromDisk {
    pub(crate) fn new(attachment: &AttachmentConfig, path: &str) -> Self {
        Self {
            path: format!("{}/data/{}", path, attachment.name),
            format: attachment.format.into(),
            file_format: attachment.file_format,
        }
    }
}

/// This component is used to load attachments from disk memory into the corresponding [`NodeAtlas`].
#[derive(Default, Component)]
pub struct AttachmentFromDiskLoader {
    pub(crate) attachments: HashMap<AttachmentIndex, AttachmentFromDisk>,
    /// Maps the id of an asset to the corresponding node id.
    handle_mapping: HashMap<HandleId, (NodeId, AttachmentIndex)>,
}

pub(crate) fn start_loading_attachment_from_disk(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<(&mut NodeAtlas, &mut AttachmentFromDiskLoader)>,
) {
    for (mut node_atlas, mut config) in terrain_query.iter_mut() {
        let NodeAtlas {
            ref mut loading_nodes,
            ref mut load_events,
            ..
        } = node_atlas.as_mut();

        let AttachmentFromDiskLoader {
            ref mut attachments,
            ref mut handle_mapping,
        } = config.as_mut();

        for &node_id in load_events.iter() {
            let node = loading_nodes.get_mut(&node_id).unwrap();

            for (
                attachment_index,
                AttachmentFromDisk {
                    ref path,
                    ref file_format,
                    ..
                },
            ) in attachments.iter()
            {
                let handle: Handle<Image> =
                    asset_server.load(&format!("{path}/{node_id}.{}", file_format.extension()));

                if asset_server.get_load_state(handle.clone()) == LoadState::Loaded {
                    node.loaded(*attachment_index);
                } else {
                    handle_mapping.insert(handle.id(), (node_id, *attachment_index));
                };

                node.set_attachment(*attachment_index, handle);
                println!("set attachment node {} {}",node_id, attachment_index);
            }
        }
    }
}

pub(crate) fn finish_loading_attachment_from_disk(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut terrain_query: Query<(&mut NodeAtlas, &mut AttachmentFromDiskLoader)>,
) {
    for event in asset_events.iter() {
        if let AssetEvent::Created { handle } = event {
            for (mut node_atlas, mut config) in terrain_query.iter_mut() {
                if let Some((node_id, attachment_index)) =
                    config.handle_mapping.remove(&handle.id())
                {
                    let image = images.get_mut(handle).unwrap();
                    let attachment = config.attachments.get(&attachment_index).unwrap();

                    image.texture_descriptor.format = attachment.format;
                    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;

                    let node = node_atlas.loading_nodes.get_mut(&node_id).unwrap();
                    node.loaded(attachment_index);
                    println!("finished loading attachment from disk {} {}",node_id,attachment_index);
                    break;
                }
            }
        }
    }
}
