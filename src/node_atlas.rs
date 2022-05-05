use crate::{
    config::{NodeId, TerrainConfig},
    quadtree::{NodeUpdate, Quadtree},
    render::gpu_node_atlas::NodeAttachmentData,
};
use bevy::{
    asset::{HandleId, LoadState},
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    utils::{HashMap, HashSet},
};
use lru::LruCache;
use std::{collections::VecDeque, mem};

type AtlasIndex = u16;

#[derive(Clone)]
pub struct NodeData {
    pub(crate) atlas_index: AtlasIndex,
    pub(crate) finished_loading: HashMap<String, bool>,
    pub(crate) node_attachments: HashMap<String, NodeAttachmentData>,
}

impl NodeData {
    pub(crate) fn load(
        id: NodeId,
        asset_server: &AssetServer,
        handle_mapping: &mut HashMap<HandleId, (NodeId, String)>,
    ) -> Self {
        // Todo: fix this mess
        let height_map: Handle<Image> = asset_server.load(&format!("output/height/{}.png", id));
        let albedo_map: Handle<Image> = asset_server.load(&format!("output/albedo/{}.png", id));

        let mut finished_loading = HashMap::new();
        let mut node_attachments = HashMap::new();

        if asset_server.get_load_state(height_map.clone()) == LoadState::Loaded {
            finished_loading.insert("height_map".into(), true);
        } else {
            finished_loading.insert("height_map".into(), false);
            handle_mapping.insert(height_map.id, (id, "height_map".into()));
        };

        if asset_server.get_load_state(albedo_map.clone()) == LoadState::Loaded {
            finished_loading.insert("albedo_map".into(), true);
        } else {
            finished_loading.insert("albedo_map".into(), false);
            handle_mapping.insert(albedo_map.id, (id, "albedo_map".into()));
        };

        node_attachments.insert(
            "height_map".into(),
            NodeAttachmentData::Texture { data: height_map },
        );
        node_attachments.insert(
            "albedo_map".into(),
            NodeAttachmentData::Texture { data: albedo_map },
        );

        Self {
            atlas_index: NodeAtlas::INACTIVE_ID,
            finished_loading,
            node_attachments,
        }
    }

    /// Returns `true` if all of the nodes attachments have finished loading.
    pub(crate) fn is_finished(&self) -> bool {
        self.finished_loading.values().all(|&finished| finished)
    }
}

/// Maps the assets to the corresponding active nodes and tracks the node updates.
#[derive(Component)]
pub struct NodeAtlas {
    pub(crate) available_indices: VecDeque<AtlasIndex>,
    /// Maps the id of an asset to the corresponding node id.
    pub(crate) handle_mapping: HashMap<HandleId, (NodeId, String)>,
    /// Stores the currently loading nodes.
    pub(crate) loading_nodes: HashMap<NodeId, NodeData>,
    /// Stores the currently active nodes.
    pub(crate) active_nodes: HashMap<NodeId, NodeData>,
    /// Caches the recently deactivated nodes.
    pub(crate) inactive_nodes: LruCache<NodeId, NodeData>,
    pub(crate) activated_nodes: Vec<NodeData>,
}

impl NodeAtlas {
    // pub(crate) const NONEXISTENT_ID: u16 = u16::MAX;
    pub(crate) const INACTIVE_ID: u16 = u16::MAX - 1;

    pub fn new(config: &TerrainConfig, cache_size: usize) -> Self {
        Self {
            available_indices: (0..config.node_atlas_size).collect(),
            handle_mapping: default(),
            loading_nodes: default(),
            active_nodes: default(),
            inactive_nodes: LruCache::new(cache_size),
            activated_nodes: default(),
        }
    }

    /// Start loading or activate all nodes ready for activation.
    pub(crate) fn activate_nodes(
        &mut self,
        nodes_to_activate: Vec<NodeId>,
        node_updates: &mut Vec<Vec<NodeUpdate>>,
        q_activated_nodes: &mut HashSet<NodeId>,
        asset_server: &AssetServer,
    ) {
        // clear the previously activated nodes
        q_activated_nodes.clear();

        let NodeAtlas {
            ref mut available_indices,
            ref mut handle_mapping,
            ref mut loading_nodes,
            ref mut active_nodes,
            ref mut inactive_nodes,
            ref mut activated_nodes,
        } = self;

        // load required nodes from cache or disk
        let mut activation_queue = nodes_to_activate
            .into_iter()
            .filter_map(|node_id| {
                if let Some(node) = inactive_nodes.pop(&node_id) {
                    // queue cached node for activation
                    Some((node_id, node))
                } else {
                    // load node before activation
                    loading_nodes.insert(
                        node_id,
                        NodeData::load(node_id, &asset_server, handle_mapping),
                    );
                    None
                }
            })
            .collect::<Vec<_>>();

        // queue all nodes, that have finished loading, for activation
        activation_queue.extend(loading_nodes.drain_filter(|_id, node| node.is_finished()));

        for (node_id, mut node) in activation_queue {
            // Todo: figure out a cleaner way of dealing with index exhaustion
            node.atlas_index = available_indices.pop_front().expect("Out of atlas ids.");

            node_updates[TerrainConfig::node_position(node_id).lod as usize].push(NodeUpdate {
                node_id,
                atlas_index: node.atlas_index as u32,
            });

            q_activated_nodes.insert(node_id); // Todo: rename this
            activated_nodes.push(node.clone()); // Todo: fix this clone
            active_nodes.insert(node_id, node);
        }
    }

    /// Deactivate all no longer required nodes.
    pub(crate) fn deactivate_nodes(
        &mut self,
        nodes_to_deactivate: Vec<NodeId>,
        node_updates: &mut Vec<Vec<NodeUpdate>>,
    ) {
        let NodeAtlas {
            ref mut available_indices,
            ref mut active_nodes,
            ref mut inactive_nodes,
            ..
        } = self;

        let deactivation_queue = nodes_to_deactivate
            .into_iter()
            .map(|node_id| (node_id, active_nodes.remove(&node_id).unwrap()));

        for (node_id, mut node) in deactivation_queue {
            available_indices.push_front(node.atlas_index);
            node.atlas_index = Self::INACTIVE_ID;

            node_updates[TerrainConfig::node_position(node_id).lod as usize].push(NodeUpdate {
                node_id,
                atlas_index: node.atlas_index as u32,
            });

            inactive_nodes.put(node_id, node);
        }
    }
}

/// Updates the node atlas according to the corresponding quadtree update.
pub fn update_nodes(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<(&mut Quadtree, &mut NodeAtlas)>,
) {
    for (mut quadtree, mut node_atlas) in terrain_query.iter_mut() {
        let Quadtree {
            ref mut activated_nodes,
            ref mut nodes_to_activate,
            ref mut nodes_to_deactivate,
            ref mut node_updates,
            ..
        } = quadtree.as_mut();

        node_atlas.deactivate_nodes(mem::take(nodes_to_deactivate), node_updates);
        node_atlas.activate_nodes(
            mem::take(nodes_to_activate),
            node_updates,
            activated_nodes,
            &asset_server,
        );
    }
}

/// Updates the load status of a node for all of it newly loaded assets.
pub fn update_load_status(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut terrain_query: Query<&mut NodeAtlas>,
) {
    for event in asset_events.iter() {
        if let AssetEvent::Created { handle } = event {
            for mut node_atlas in terrain_query.iter_mut() {
                if let Some((id, label)) = node_atlas.handle_mapping.remove(&handle.id) {
                    let image = images.get_mut(handle).unwrap();

                    if label == "height_map" {
                        image.texture_descriptor.format = TextureFormat::R16Unorm;
                        image.texture_descriptor.usage = TextureUsages::COPY_SRC
                            | TextureUsages::COPY_DST
                            | TextureUsages::TEXTURE_BINDING;
                    }
                    if label == "albedo_map" {
                        image.texture_descriptor.usage = TextureUsages::COPY_SRC
                            | TextureUsages::COPY_DST
                            | TextureUsages::TEXTURE_BINDING;
                    }

                    let node = node_atlas.loading_nodes.get_mut(&id).unwrap();
                    node.finished_loading.insert(label, true);

                    break;
                }
            }
        }
    }
}
