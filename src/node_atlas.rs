use crate::{
    config::{NodeId, TerrainConfig},
    quadtree::{NodeUpdate, Quadtree},
};
use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use lru::LruCache;
use std::{collections::VecDeque, mem};

/// It is emitted whenever the [`NodeAtlas`] requests loading a node, for it to become active.
pub struct LoadNodeEvent(pub NodeId);

/// Identifier of an active node inside the
/// [`GpuNodeAtlas`](crate::render::gpu_node_atlas::GpuNodeAtlas).
pub type AtlasIndex = u16;

/// Stores the data, which will be loaded into the corresponding
/// [`AtlasAttachment`](crate::render::gpu_node_atlas::AtlasAttachment) once the node
/// becomes activated.
#[derive(Clone)]
pub enum NodeAttachment {
    Buffer { data: Vec<u8> },
    Texture { handle: Handle<Image> },
}

/// Stores all of the [`NodeAttachment`]s of the node, alongside their loading state.
#[derive(Clone)]
pub struct NodeData {
    /// The [`AtlasIndex`] of the node.
    pub(crate) atlas_index: AtlasIndex,
    /// Stores all of the [`NodeAttachment`]s of the node.
    pub(crate) attachment_data: HashMap<String, NodeAttachment>,
    /// The set of still loading [`NodeAttachment`]s. Is empty if the node is fully loaded.
    loading_attachments: HashSet<String>, // Todo: maybe factor this out?
}

impl NodeData {
    /// Sets the attachment data of the node.
    pub fn set_attachment(&mut self, label: String, attachment: NodeAttachment) {
        self.attachment_data.insert(label.clone(), attachment);
    }

    /// Marks the corresponding [`NodeAttachment`] as loaded.
    pub fn loaded(&mut self, label: &String) {
        self.loading_attachments.remove(label);
    }
}

/// Orchestrates the loading and activation/deactivation process of all nodes.
///
/// It activates and deactivates nodes according to the decisions of the [`Quadtree`].
/// Recently deactivated nodes are cached for prompt reactivation, otherwise nodes have to be loaded
/// first. This happens by sending a [`LoadNodeEvent`] for which [`NodeAttachment`]-loading-systems
/// can listen.
///
/// Each activated node gets assigned a unique [`AtlasIndex`] for accessing the attached data in
/// the terrain shader.
#[derive(Component)]
pub struct NodeAtlas {
    /// Stores the atlas indices, which are not currently taken by the active nodes.
    pub(crate) available_indices: VecDeque<AtlasIndex>,
    /// Specifies which [`NodeAttachment`]s to load for each node.
    pub(crate) attachments_to_load: HashSet<String>,
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
    pub(crate) const INACTIVE_INDEX: AtlasIndex = AtlasIndex::MAX - 1;

    /// Creates a new node atlas based on the supplied [`TerrainConfig`].
    pub fn new(config: &TerrainConfig) -> Self {
        Self {
            available_indices: (0..config.node_atlas_size).collect(),
            attachments_to_load: config.attachments_to_load(),
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
        load_events: &mut EventWriter<LoadNodeEvent>,
    ) {
        let NodeAtlas {
            ref mut available_indices,
            ref attachments_to_load,
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
                    load_events.send(LoadNodeEvent(node_id));
                    loading_nodes.insert(
                        node_id,
                        NodeData {
                            atlas_index: NodeAtlas::INACTIVE_INDEX,
                            attachment_data: HashMap::new(),
                            loading_attachments: attachments_to_load.clone(),
                        },
                    );
                    None
                }
            })
            .collect::<Vec<_>>();

        // queue all nodes, that have finished loading, for activation
        activation_queue
            .extend(loading_nodes.drain_filter(|_id, node| node.loading_attachments.is_empty()));

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
            node.atlas_index = Self::INACTIVE_INDEX;

            node_updates[TerrainConfig::node_position(node_id).lod as usize].push(NodeUpdate {
                node_id,
                atlas_index: node.atlas_index as u32,
            });

            inactive_nodes.put(node_id, node);
        }
    }
}

/// Updates the node atlas according to the corresponding quadtree update.
pub(crate) fn update_nodes(
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
