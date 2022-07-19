use crate::terrain::AttachmentIndex;
use crate::{
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
    pub(crate) attachments: HashMap<AttachmentIndex, Handle<Image>>, // Todo: maybe replace with array or vec?
    /// The set of still loading [`NodeAttachment`]s. Is empty if the node is fully loaded.
    loading_attachments: HashSet<AttachmentIndex>,
}

impl LoadingNode {
    /// Sets the attachment data of the node.
    pub fn set_attachment(&mut self, attachment_index: AttachmentIndex, attachment: Handle<Image>) {
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
    pub(crate) _attachments: HashMap<AttachmentIndex, Handle<Image>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadingState {
    Loading,
    Loaded,
}

/// Stores the state of a present node in the [`NodeAtlas`].
pub(crate) struct AtlasNode {
    /// Indicates whether or not the node is loading or loaded.
    pub(crate) state: LoadingState,
    /// The index of the node inside the atlas.
    pub(crate) atlas_index: AtlasIndex,
    /// The count of [`Quadtrees`] that have demanded this node.
    dependents: u32,
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

    attachment_count: usize,
    /// Stores the cpu accessible data of all present nodes.
    pub(crate) data: Vec<NodeData>, // Todo: build api for accessing data on the cpu
    /// Stores the states of all present nodes.
    pub(crate) nodes: HashMap<NodeId, AtlasNode>, // Todo: change to hash set
    /// Lists the unused nodes in least recently used order.
    unused_nodes: VecDeque<UnusedNode>,
}

impl NodeAtlas {
    /// Creates a new node atlas based on the supplied [`TerrainConfig`].
    pub fn new(config: &TerrainConfig) -> Self {
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
            attachment_count: config.attachments.len(),
            unused_nodes,
            data,
            nodes: default(),
        }
    }

    /// Adjusts the nodes atlas according to the requested and released nodes of the [`Quadtree`]
    /// and provides it with the available atlas indices.
    fn fulfill_request(&mut self, quadtree: &mut Quadtree) {
        let NodeAtlas {
            attachment_count,
            unused_nodes,
            nodes,
            loading_nodes,
            load_events,
            ..
        } = self;

        // release nodes that are on longer required
        for node_id in quadtree.released_nodes.drain(..) {
            let node = nodes
                .get_mut(&node_id)
                .expect("Tried releasing a node, which is not present.");
            node.dependents -= 1;

            if node.dependents == 0 {
                // the node is not used anymore
                unused_nodes.push_back(UnusedNode {
                    node_id,
                    atlas_index: node.atlas_index,
                });
            }
        }

        // load nodes that are requested
        for node_id in quadtree.demanded_nodes.drain(..) {
            // check if the node is already present else start loading it
            if let Some(node) = nodes.get_mut(&node_id) {
                if node.dependents == 0 {
                    // the node is now used again
                    unused_nodes.retain(|unused_node| node.atlas_index != unused_node.atlas_index);
                }

                node.dependents += 1;
            } else {
                // Todo: implement better loading strategy
                // remove least recently used node and reuse its atlas index
                let unused_node = unused_nodes.pop_front().expect("Atlas out of indices");

                nodes.remove(&unused_node.node_id);
                nodes.insert(
                    node_id,
                    AtlasNode {
                        dependents: 1,
                        state: LoadingState::Loading,
                        atlas_index: unused_node.atlas_index,
                    },
                );

                // start loading the node
                load_events.push(node_id);
                loading_nodes.insert(
                    node_id,
                    LoadingNode {
                        atlas_index: unused_node.atlas_index,
                        loading_attachments: (0..*attachment_count).collect(),
                        attachments: default(),
                    },
                );
            }
        }
    }

    /// Checks all nodes that have finished loading, marks them accordingly and prepares the data
    /// to be send to the gpu by the [`GpuNodeAtlas`](crate::render::gpu_node_atlas::GpuNodeAtlas).
    fn update_loaded_nodes(&mut self) {
        let NodeAtlas {
            ref mut data,
            ref mut load_events,
            ref mut nodes,
            ref mut loading_nodes,
            ref mut loaded_nodes,
            ..
        } = self;

        load_events.clear();

        // update all nodes that have finished loading
        for (node_id, loading_node) in loading_nodes.drain_filter(|_, node| node.finished_loading())
        {
            if let Some(node) = nodes.get_mut(&node_id) {
                node.state = LoadingState::Loaded;

                // Todo: only keep attachments required by the CPU around
                data[node.atlas_index as usize] = NodeData {
                    _attachments: loading_node.attachments.clone(),
                };

                loaded_nodes.push(loading_node);
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
                node_atlas.fulfill_request(quadtree);
            }
        }
    }
}
