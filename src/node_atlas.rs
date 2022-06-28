use crate::{
    attachment::{AtlasAttachmentConfig, AttachmentIndex, NodeAttachment},
    quadtree::{NodeId, Quadtree, INVALID_NODE_ID},
    terrain::{Terrain, TerrainConfig},
    TerrainView, TerrainViewComponents,
};
use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use std::collections::VecDeque;

/// Identifier of an active node (and its attachments) inside the node atlas.
pub type AtlasIndex = u16;

/// An invalid [`AtlasIndex`], which is used for initialization.
pub(crate) const INVALID_ATLAS_INDEX: AtlasIndex = AtlasIndex::MAX;

/// Stores all of the [`NodeAttachment`]s of the node, alongside their loading state.
#[derive(Clone)]
pub struct LoadingNode {
    pub(crate) atlas_index: AtlasIndex,
    /// Stores all of the [`NodeAttachment`]s of the node.
    pub(crate) attachments: HashMap<AttachmentIndex, NodeAttachment>,
    /// The set of still loading [`NodeAttachment`]s. Is empty if the node is fully loaded.
    loading_attachments: HashSet<AttachmentIndex>,
}

impl LoadingNode {
    /// Sets the attachment data of the node.
    pub fn attachment(&mut self, attachment_index: AttachmentIndex, attachment: NodeAttachment) {
        self.attachments.insert(attachment_index, attachment);
    }

    /// Marks the corresponding [`NodeAttachment`] as loaded.
    pub fn loaded(&mut self, attachment_index: AttachmentIndex) {
        self.loading_attachments.remove(&attachment_index);
    }

    /// Returns whether all node attachments of the node have finished loading.
    fn finished_loading(&self) -> bool {
        self.loading_attachments.is_empty()
    }
}

/// Stores all of the cpu accessible [`NodeAttachment`]s of the node, after it has been loaded.
#[derive(Default)]
pub struct NodeData {
    /// Stores all of the cpu accessible [`NodeAttachment`]s of the node.
    pub(crate) _attachments: HashMap<AttachmentIndex, NodeAttachment>,
}

/// Stores the state of a present node in the [`NodeAtlas`].
struct NodeState {
    /// The count of [`Quadtrees`] that have requested this node.
    requests: u32,
    /// Indicates whether or not the node is loading or loaded.
    loading: bool,
    /// The index of the node inside the atlas.
    atlas_index: AtlasIndex,
}

/// A Node which is not currently requested by any [`Quadtree`].
struct UnusedNode {
    node_id: NodeId,
    atlas_index: AtlasIndex,
}

/// Orchestrates the loading process of all nodes according to the decisions of the [`Quadtree`]s.
///
/// A node is considered present and assigned an [`AtlasIndex`] as soon as it is
/// requested by any quadtree. Then the node atlas will start loading all of its [`NodeAttachment`]s
/// by sending a [`LoadNodeEvent`] for which attachment-loading-systems can listen.
/// Nodes that are not being used by any quadtree anymore are cached (LRU),
/// until new atlas indices are required.
///
/// The [`AtlasIndex`] can be used for accessing the attached data in systems by the CPU
/// and in shaders by the GPU.
#[derive(Component)]
pub struct NodeAtlas {
    /// Nodes that are requested to be loaded this frame.
    pub load_events: Vec<NodeId>,
    /// Stores the nodes, that have finished loading this frame.
    pub(crate) loaded_nodes: Vec<LoadingNode>,
    /// Stores the currently loading nodes.
    pub(crate) loading_nodes: HashMap<NodeId, LoadingNode>,
    /// Specifies which [`NodeAttachment`]s to load for each node.
    attachments_to_load: HashSet<AttachmentIndex>,
    /// Lists the unused nodes in least recently used order.
    unused_nodes: VecDeque<UnusedNode>,
    /// Stores the cpu accessible data of all present nodes.
    pub(crate) data: Vec<NodeData>, // Todo: build api for accessing data on the cpu
    /// Stores the states of all present nodes.
    node_states: HashMap<NodeId, NodeState>,
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

        let mut data = Vec::with_capacity(config.node_atlas_size as usize);
        let mut unused_nodes = VecDeque::with_capacity(config.node_atlas_size as usize);

        for atlas_index in 0..config.node_atlas_size {
            data.push(default());
            unused_nodes.push_back(UnusedNode {
                node_id: INVALID_NODE_ID,
                atlas_index,
            });
        }

        Self {
            load_events: default(),
            loaded_nodes: default(),
            loading_nodes: default(),
            attachments_to_load,
            unused_nodes,
            data,
            node_states: default(),
        }
    }

    /// Adjusts the nodes atlas according to the requested and released nodes of the [`Quadtree`]
    /// and provides it with the available atlas indices.
    fn adjust_to_quadtree(&mut self, quadtree: &mut Quadtree) {
        let NodeAtlas {
            ref attachments_to_load,
            ref mut unused_nodes,
            ref mut node_states,
            ref mut loading_nodes,
            ref mut load_events,
            ..
        } = self;

        // release nodes that are on longer required
        for node_id in quadtree.released_nodes.drain(..) {
            let state = node_states
                .get_mut(&node_id)
                .expect("Tried releasing a node, which is not present.");
            state.requests -= 1;

            if state.requests == 0 {
                // the node is not used anymore
                unused_nodes.push_back(UnusedNode {
                    node_id,
                    atlas_index: state.atlas_index,
                });
            }
        }

        // load nodes that are requested
        for node_id in quadtree.requested_nodes.drain(..) {
            // check if the node is already present else start loading it
            if let Some(state) = node_states.get_mut(&node_id) {
                if state.requests == 0 {
                    // the node is now used again
                    unused_nodes.retain(|node| node.atlas_index != state.atlas_index);
                }

                state.requests += 1;
            } else {
                // remove least recently used node and reuse its atlas index
                let unused_node = unused_nodes.pop_front().expect("Atlas out of indices");

                node_states.remove(&unused_node.node_id);
                node_states.insert(
                    node_id,
                    NodeState {
                        requests: 1,
                        loading: true,
                        atlas_index: unused_node.atlas_index,
                    },
                );

                // start loading the node
                load_events.push(node_id);
                loading_nodes.insert(
                    node_id,
                    LoadingNode {
                        atlas_index: unused_node.atlas_index,
                        loading_attachments: attachments_to_load.clone(),
                        attachments: default(),
                    },
                );
            }
        }
    }

    pub(crate) fn update_quadtree(&mut self, quadtree: &mut Quadtree) {
        let Quadtree {
            ref mut waiting_nodes,
            ref mut provided_nodes,
            ..
        } = quadtree;

        // provide nodes that have finished loading
        waiting_nodes.retain(|&node_id| {
            if let Some(state) = self.node_states.get_mut(&node_id) {
                if !state.loading {
                    provided_nodes.insert(node_id, state.atlas_index);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });
    }

    /// Checks all nodes that have finished loading, marks them accordingly and prepares the data
    /// to be send to the gpu by the [`GpuNodeAtlas`](crate::render::gpu_node_atlas::GpuNodeAtlas).
    fn update_loaded_nodes(&mut self) {
        let NodeAtlas {
            ref mut data,
            ref mut load_events,
            ref mut node_states,
            ref mut loading_nodes,
            ref mut loaded_nodes,
            ..
        } = self;

        load_events.clear();

        // update all nodes that have finished loading
        for (node_id, node) in loading_nodes.drain_filter(|_, node| node.finished_loading()) {
            if let Some(state) = node_states.get_mut(&node_id) {
                state.loading = false;

                // Todo: only keep attachments required by the CPU around
                data[state.atlas_index as usize] = NodeData {
                    _attachments: node.attachments.clone(),
                };

                loaded_nodes.push(node);
            } else {
                dbg!("Dropped node after loading.");
                // node no longer required, can safely be ignored
            }
        }
    }
}

/// Updates the node atlas according to all corresponding quadtrees.
pub(crate) fn update_node_atlas(
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    mut terrain_query: Query<(Entity, &mut NodeAtlas), With<Terrain>>,
) {
    for (terrain, mut node_atlas) in terrain_query.iter_mut() {
        node_atlas.update_loaded_nodes();

        for view in view_query.iter() {
            if let Some(quadtree) = quadtrees.get_mut(&(terrain, view)) {
                node_atlas.adjust_to_quadtree(quadtree);
            }
        }
    }
}
