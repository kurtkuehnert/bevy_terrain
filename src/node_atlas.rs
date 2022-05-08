use crate::{
    attachments::NodeAttachmentData,
    config::{NodeId, TerrainConfig},
    quadtree::{NodeUpdate, Quadtree},
};
use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use lru::LruCache;
use std::{collections::VecDeque, mem};

pub struct LoadNodeEvent(pub NodeId);

type AtlasIndex = u16;

#[derive(Clone)]
pub struct NodeData {
    pub(crate) atlas_index: AtlasIndex,
    pub(crate) loading_attachments: HashSet<String>,
    pub(crate) attachment_data: HashMap<String, NodeAttachmentData>,
}

impl NodeData {
    pub(crate) fn new(attachments: HashSet<String>) -> Self {
        Self {
            atlas_index: NodeAtlas::INACTIVE_ID,
            loading_attachments: attachments,
            attachment_data: default(),
        }
    }

    /// Returns `true` if all of the nodes attachments have finished loading.
    pub(crate) fn is_finished(&self) -> bool {
        self.loading_attachments.is_empty()
    }
}

/// Maps the assets to the corresponding active nodes and tracks the node updates.
#[derive(Component)]
pub struct NodeAtlas {
    /// Stores the atlas indices, which are not currently taken by the active nodes.
    pub(crate) available_indices: VecDeque<AtlasIndex>,
    /// Specifies which [`NodeAttachmentData`] to load for each node.
    pub(crate) attachments: HashSet<String>,
    /// Stores the currently loading nodes.
    pub(crate) loading_nodes: HashMap<NodeId, NodeData>,
    /// Stores the currently active nodes.
    pub(crate) active_nodes: HashMap<NodeId, NodeData>,
    /// Caches the recently deactivated nodes.
    pub(crate) inactive_nodes: LruCache<NodeId, NodeData>,
    /// Stores the nodes, that where activated this frame.
    pub(crate) activated_nodes: Vec<NodeData>,
}

impl NodeAtlas {
    // pub(crate) const NONEXISTENT_ID: u16 = u16::MAX;
    pub(crate) const INACTIVE_ID: u16 = u16::MAX - 1;

    pub fn new(config: &TerrainConfig) -> Self {
        let mut attachments = config
            .attachments
            .keys()
            .map(|label| label.clone())
            .collect::<HashSet<_>>();

        Self {
            available_indices: (0..config.node_atlas_size).collect(),
            attachments,
            loading_nodes: default(),
            active_nodes: default(),
            inactive_nodes: LruCache::new(config.cache_size),
            activated_nodes: default(),
        }
    }

    /// Start loading or activate all nodes ready for activation.
    pub(crate) fn activate_nodes(
        &mut self,
        nodes_to_activate: Vec<NodeId>,
        node_updates: &mut Vec<Vec<NodeUpdate>>,
        nodes_activated: &mut HashSet<NodeId>,
        mut load_events: &mut EventWriter<LoadNodeEvent>,
    ) {
        let NodeAtlas {
            ref mut available_indices,
            ref mut attachments,
            ref mut loading_nodes,
            ref mut active_nodes,
            ref mut inactive_nodes,
            ref mut activated_nodes,
            ..
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
                    load_events.send(LoadNodeEvent(node_id)); // Todo: differentiate between different node atlases
                    loading_nodes.insert(node_id, NodeData::new(attachments.clone()));
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

            nodes_activated.insert(node_id);
            activated_nodes.push(node.clone());
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
    mut load_events: EventWriter<LoadNodeEvent>,
    mut terrain_query: Query<(&mut Quadtree, &mut NodeAtlas)>,
) {
    for (mut quadtree, mut node_atlas) in terrain_query.iter_mut() {
        let Quadtree {
            ref mut nodes_activated,
            ref mut nodes_to_activate,
            ref mut nodes_to_deactivate,
            ref mut node_updates,
            ..
        } = quadtree.as_mut();

        node_atlas.deactivate_nodes(mem::take(nodes_to_deactivate), node_updates);
        node_atlas.activate_nodes(
            mem::take(nodes_to_activate),
            node_updates,
            nodes_activated,
            &mut load_events,
        );
    }
}
