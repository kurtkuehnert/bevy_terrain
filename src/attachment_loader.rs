use crate::terrain::AttachmentIndex;
use crate::{node_atlas::NodeAtlas, quadtree::NodeId};
use bevy::{
    asset::{AssetServer, HandleId, LoadState},
    prelude::*,
    render::render_resource::*,
    utils::HashMap,
};

pub struct AttachmentFromDisk {
    pub path: String,
    pub format: TextureFormat,
}

#[derive(Default, Component)]
pub struct AttachmentFromDiskLoader {
    pub attachments: HashMap<AttachmentIndex, AttachmentFromDisk>,
    /// Maps the id of an asset to the corresponding node id.
    pub handle_mapping: HashMap<HandleId, (NodeId, AttachmentIndex)>,
}

impl AttachmentFromDiskLoader {
    pub fn add_attachment(
        &mut self,
        attachment_index: AttachmentIndex,
        attachment: AttachmentFromDisk,
    ) {
        self.attachments.insert(attachment_index, attachment);
    }
}

pub fn start_loading_attachment_from_disk(
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

            for (attachment_index, AttachmentFromDisk { ref path, .. }) in attachments.iter() {
                let handle: Handle<Image> = asset_server.load(&format!("{path}/{node_id}.png"));

                if asset_server.get_load_state(handle.clone()) == LoadState::Loaded {
                    node.loaded(*attachment_index);
                } else {
                    handle_mapping.insert(handle.id, (node_id, *attachment_index));
                };

                node.set_attachment(*attachment_index, handle);
            }
        }
    }
}

pub fn finish_loading_attachment_from_disk(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut terrain_query: Query<(&mut NodeAtlas, &mut AttachmentFromDiskLoader)>,
) {
    for event in asset_events.iter() {
        if let AssetEvent::Created { handle } = event {
            for (mut node_atlas, mut config) in terrain_query.iter_mut() {
                if let Some((node_id, attachment_index)) = config.handle_mapping.remove(&handle.id)
                {
                    let image = images.get_mut(handle).unwrap();
                    let attachment = config.attachments.get(&attachment_index).unwrap();

                    image.texture_descriptor.format = attachment.format;
                    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;

                    let node = node_atlas.loading_nodes.get_mut(&node_id).unwrap();
                    node.loaded(attachment_index);
                    break;
                }
            }
        }
    }
}
