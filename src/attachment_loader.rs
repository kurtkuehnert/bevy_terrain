//! The default attachment loader, which loads node data from disk.

use crate::{
    plugin::TerrainPluginConfig,
    preprocess::{Preprocessor, TileConfig},
    terrain_data::{node_atlas::NodeAtlas, AttachmentConfig, AttachmentIndex, FileFormat, NodeId},
};
use bevy::{
    asset::{AssetServer, LoadState},
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
#[derive(Component)]
pub struct AttachmentFromDiskLoader {
    pub(crate) attachments: HashMap<AttachmentIndex, AttachmentFromDisk>,
    /// Maps the asset id of an image to the corresponding node id.
    id_mapping: HashMap<AssetId<Image>, (NodeId, AttachmentIndex)>,
    preprocessor: Preprocessor,
    path: String,
}

impl AttachmentFromDiskLoader {
    pub fn new(lod_count: u32, path: String) -> Self {
        Self {
            attachments: Default::default(),
            id_mapping: Default::default(),
            preprocessor: Preprocessor::new(lod_count, path.clone()),
            path,
        }
    }

    pub fn add_base_attachment(&mut self, plugin_config: &TerrainPluginConfig, tile: TileConfig) {
        self.attachments.insert(
            0,
            AttachmentFromDisk::new(&plugin_config.base.height_attachment(), &self.path),
        );
        self.attachments.insert(
            1,
            AttachmentFromDisk::new(&plugin_config.base.minmax_attachment(), &self.path),
        );

        self.preprocessor.base = Some((tile, plugin_config.base));
    }

    pub fn add_attachment(
        &mut self,
        plugin_config: &TerrainPluginConfig,
        tile: TileConfig,
        attachment_index: usize,
    ) {
        let attachment = plugin_config.attachments[attachment_index].clone();

        self.attachments.insert(
            attachment_index,
            AttachmentFromDisk::new(&attachment, &self.path),
        );

        self.preprocessor.attachments.push((tile, attachment));
    }

    pub fn preprocess(&self) {
        self.preprocessor.preprocess();
    }
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
            ref mut id_mapping,
            ..
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

                if asset_server.load_state(&handle) == LoadState::Loaded {
                    node.loaded(*attachment_index);
                } else {
                    id_mapping.insert(handle.id(), (node_id, *attachment_index));
                };

                node.set_attachment(*attachment_index, handle);
            }
        }
    }
}

pub(crate) fn finish_loading_attachment_from_disk(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut terrain_query: Query<(&mut NodeAtlas, &mut AttachmentFromDiskLoader)>,
) {
    for &event in asset_events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            for (mut node_atlas, mut config) in terrain_query.iter_mut() {
                if let Some((node_id, attachment_index)) = config.id_mapping.remove(&id) {
                    let image = images.get_mut(id).unwrap();
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
