use crate::{
    attachment::{AtlasAttachmentConfig, AttachmentIndex, NodeAttachment},
    config::TerrainConfig,
    quadtree::{NodeId, Quadtree, INVALID_NODE_ID},
};
use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use std::collections::VecDeque;

/// It is emitted whenever the [`NodeAtlas`] requests loading a node, for it to become active.
pub struct LoadNodeEvent(pub NodeId);

/// Identifier of an active node (and its attachments) inside the node atlas.
pub type AtlasIndex = u16;

pub(crate) const INVALID_ATLAS_INDEX: AtlasIndex = AtlasIndex::MAX;

/// Stores all of the [`NodeAttachment`]s of the node, alongside their loading state.
#[derive(Clone)]
pub struct LoadingNode {
    /// The set of still loading [`NodeAttachment`]s. Is empty if the node is fully loaded.
    loading_attachments: HashSet<AttachmentIndex>,
    /// Stores all of the [`NodeAttachment`]s of the node.
    pub(crate) attachments: HashMap<AttachmentIndex, NodeAttachment>,
}

impl LoadingNode {
    /// Sets the attachment data of the node.
    pub fn set_attachment(
        &mut self,
        attachment_index: AttachmentIndex,
        attachment: NodeAttachment,
    ) {
        self.attachments.insert(attachment_index, attachment);
    }

    /// Marks the corresponding [`NodeAttachment`] as loaded.
    pub fn loaded(&mut self, attachment_index: AttachmentIndex) {
        self.loading_attachments.remove(&attachment_index);
    }

    fn is_loaded(&self) -> bool {
        self.loading_attachments.is_empty()
    }
}

/// Stores all of the cpu accessible [`NodeAttachment`]s of the node, after it has been loaded.
#[derive(Clone, Default)]
pub struct NodeData {
    /// Stores all of the cpu accessible [`NodeAttachment`]s of the node.
    pub(crate) attachments: HashMap<AttachmentIndex, NodeAttachment>,
}

pub struct NodeState {
    requests: u32,
    loading: bool,
    atlas_index: AtlasIndex,
}

/// Orchestrates the loading and activation/deactivation process of all nodes.
///
/// It activates and deactivates nodes according to the decisions of the [`Quadtree`].
/// Recently deactivated nodes are cached for prompt reactivation, otherwise nodes have to be loaded
/// first. This happens by sending a [`LoadNodeEvent`] for which [`NodeAttachment`]-loading-systems
/// can listen.
///
/// Each activated node gets assigned a unique [`AtlasIndex`] for accessing the attached data
/// in systems by the CPU and in shaders by the GPU.
#[derive(Component)]
pub struct NodeAtlas {
    /// Stores the cpu accessible data of all present nodes.
    pub(crate) data: Vec<(NodeId, NodeData)>, // Todo: build api for accessing data on the cpu
    /// Stores the nodes, that have finished loading this frame.
    pub(crate) loaded_nodes: Vec<(AtlasIndex, LoadingNode)>,
    /// Specifies which [`NodeAttachment`]s to load for each node.
    attachments_to_load: HashSet<AttachmentIndex>,
    /// Lists the least recently used atlas indices, which are not used by any node.
    available_indices: VecDeque<AtlasIndex>,
    /// Stores the states of all present nodes.
    node_states: HashMap<NodeId, NodeState>,
    /// Stores the currently loading nodes.
    pub(crate) loading_nodes: HashMap<NodeId, LoadingNode>,
}

impl NodeAtlas {
    /// Creates a new node atlas based on the supplied [`TerrainConfig`].
    pub fn new(config: &TerrainConfig) -> Self {
        let attachments_to_load = config
            .attachments
            .iter()
            .filter_map(|(&attachment_index, config)| match config {
                AtlasAttachmentConfig::Sampler { .. } => None,
                _ => Some(attachment_index),
            })
            .collect();

        let data = vec![(INVALID_NODE_ID, NodeData::default()); config.node_atlas_size as usize];
        let available_indices = (0..config.node_atlas_size).collect();

        Self {
            data,
            loaded_nodes: default(),
            attachments_to_load,
            available_indices,
            node_states: default(),
            loading_nodes: default(),
        }
    }

    fn adjust_to_quadtree(
        &mut self,
        quadtree: &mut Quadtree,
        load_events: &mut EventWriter<LoadNodeEvent>,
    ) {
        let NodeAtlas {
            ref data,
            ref attachments_to_load,
            ref mut available_indices,
            ref mut node_states,
            ref mut loading_nodes,
            ..
        } = self;

        let Quadtree {
            ref mut released_nodes,
            ref mut requested_nodes,
            ref mut waiting_nodes,
            ref mut provided_nodes,
            ..
        } = quadtree;

        // release nodes that are on longer required
        for node_id in released_nodes.drain(..) {
            if let Some(state) = node_states.get_mut(&node_id) {
                state.requests -= 1;

                if state.requests == 0 {
                    // the node is not used anymore
                    available_indices.push_back(state.atlas_index);
                }
            } else {
                dbg!(node_id);
            }
        }

        // load nodes that are requested
        for node_id in requested_nodes.drain(..) {
            // check if the node is already present else start loading it
            if let Some(state) = node_states.get_mut(&node_id) {
                if state.requests == 0 {
                    // the node is now used again
                    available_indices.retain(|&atlas_index| atlas_index != state.atlas_index);
                }

                state.requests += 1;
            } else {
                // start loading the node
                load_events.send(LoadNodeEvent(node_id));
                loading_nodes.insert(
                    node_id,
                    LoadingNode {
                        loading_attachments: attachments_to_load.clone(),
                        attachments: default(),
                    },
                );

                // remove least recently used node and reuse its atlas index
                let atlas_index = available_indices.pop_front().expect("Atlas out of indices");
                node_states.remove(&data[atlas_index as usize].0);
                node_states.insert(
                    node_id,
                    NodeState {
                        requests: 1,
                        loading: true,
                        atlas_index,
                    },
                );
            }
        }

        // provide nodes that have finished loading
        waiting_nodes.retain(|&node_id| {
            if let Some(state) = node_states.get_mut(&node_id) {
                if !state.loading {
                    provided_nodes.push((node_id, state.atlas_index));
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });
    }

    fn update_loaded(&mut self) {
        let NodeAtlas {
            ref mut data,
            ref mut node_states,
            ref mut loading_nodes,
            ref mut loaded_nodes,
            ..
        } = self;

        // update all nodes that have finished loading
        for (node_id, node) in loading_nodes.drain_filter(|_, node| node.is_loaded()) {
            if let Some(state) = node_states.get_mut(&node_id) {
                state.loading = false;

                data.insert(
                    state.atlas_index as usize,
                    (
                        node_id,
                        NodeData {
                            attachments: node.attachments.clone(),
                        },
                    ),
                );
                loaded_nodes.push((state.atlas_index, node))
            } else {
                // node no longer required
            }
        }
    }
}

/// Updates the node atlas according to all corresponding quadtrees.
pub(crate) fn update_node_atlas(
    mut load_events: EventWriter<LoadNodeEvent>,
    mut terrain_query: Query<(&mut Quadtree, &mut NodeAtlas)>,
) {
    for (mut quadtree, mut node_atlas) in terrain_query.iter_mut() {
        node_atlas.update_loaded();
        node_atlas.adjust_to_quadtree(&mut quadtree, &mut load_events);
    }
}
