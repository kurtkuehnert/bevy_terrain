use crate::{
    terrain::{Terrain, TerrainConfig},
    terrain_data::{
        quadtree::Quadtree, AtlasAttachment, AtlasIndex, AttachmentIndex, NodeId, INVALID_NODE_ID,
    },
    TerrainView, TerrainViewComponents,
};
use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use std::collections::VecDeque;

/// Stores all of the attachments of the node, alongside their loading state.
#[derive(Clone)]
pub struct LoadingNode {
    /// The atlas index of the node.
    pub(crate) atlas_index: AtlasIndex,
    // Todo: replace with array or vec of options
    /// Stores all of the nodes attachments.
    pub(crate) attachments: HashMap<AttachmentIndex, Handle<Image>>,
    /// The set of still loading attachments. Is empty if the node is fully loaded.
    loading_attachments: HashSet<AttachmentIndex>,
}

impl LoadingNode {
    /// Sets the attachment data of the node.
    pub fn set_attachment(&mut self, attachment_index: AttachmentIndex, attachment: Handle<Image>) {
        self.attachments.insert(attachment_index, attachment);
    }

    /// Marks the corresponding attachment as loaded.
    pub fn loaded(&mut self, attachment_index: AttachmentIndex) {
        self.loading_attachments.remove(&attachment_index);
    }

    /// Returns whether all node attachments of the node have finished loading.
    fn finished_loading(&self) -> bool {
        self.loading_attachments.is_empty()
    }
}

/// Stores all of the cpu accessible attachments of the node, after it has been loaded.
#[derive(Clone, Default)]
pub struct NodeData {
    // Todo: replace with array or vec of options
    /// Stores all of the cpu accessible attachments of the node.
    pub(crate) _attachments: HashMap<AttachmentIndex, Handle<Image>>,
}

/// The current state of a node of a [`NodeAtlas`].
///
/// This indicates, whether the node is loading or loaded and ready to be used.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadingState {
    /// The node is loading, but can not be used yet.
    Loading,
    /// The node is loaded and can be used.
    Loaded,
}

/// The internal representation of a present node in a [`NodeAtlas`].
pub(crate) struct AtlasNode {
    /// Indicates whether or not the node is loading or loaded.
    pub(crate) state: LoadingState,
    /// The index of the node inside the atlas.
    pub(crate) atlas_index: AtlasIndex,
    /// The count of [`Quadtrees`] that have requested this node.
    requests: u32,
}

/// A node which is not currently requested by any [`Quadtree`].
struct UnusedNode {
    node_id: NodeId,
    atlas_index: AtlasIndex,
}

/// A sparse storage of all terrain attachments, which streams data in and out of memory
/// depending on the decisions of the corresponding [`Quadtree`]s.
///
/// A node is considered present and assigned an [`AtlasIndex`] as soon as it is
/// requested by any quadtree. Then the node atlas will start loading all of its attachments
/// by storing the [`NodeId`] (for one frame) in `load_events` for which attachment-loading-systems
/// can listen.
/// Nodes that are not being used by any quadtree anymore are cached (LRU),
/// until new atlas indices are required.
///
/// The [`AtlasIndex`] can be used for accessing the attached data in systems by the CPU
/// and in shaders by the GPU.
#[derive(Component)]
pub struct NodeAtlas {
    /// Nodes that are requested to be loaded this frame.
    pub load_events: Vec<NodeId>,
    /// Stores the cpu accessible data of all loaded nodes.
    pub(crate) data: Vec<NodeData>, // Todo: build api for accessing data on the cpu
    /// Stores the atlas attachments of the terrain.
    pub(crate) attachments: Vec<AtlasAttachment>,
    /// Stores the nodes, that have finished loading this frame.
    /// This data will be send to the
    /// [`GpuNodeAtlas`](super::gpu_node_atlas::GpuNodeAtlas) each frame.
    pub(crate) loaded_nodes: Vec<LoadingNode>,
    /// Stores the currently loading nodes.
    pub(crate) loading_nodes: HashMap<NodeId, LoadingNode>,
    /// The amount of nodes the can be loaded simultaneously in the node atlas.
    pub(crate) size: u16,
    /// Stores the states of all present nodes.
    pub(crate) nodes: HashMap<NodeId, AtlasNode>,
    pub(crate) existing_nodes: HashSet<NodeId>,
    /// Lists the unused nodes in least recently used order.
    unused_nodes: VecDeque<UnusedNode>,
}

impl NodeAtlas {
    /// Creates a new quadtree from parameters.
    ///
    /// * `size` - The amount of nodes the can be loaded simultaneously in the node atlas.
    /// * `attachments` - The atlas attachments of the terrain.
    pub fn new(
        size: u16,
        attachments: Vec<AtlasAttachment>,
        existing_nodes: HashSet<NodeId>,
    ) -> Self {
        let unused_nodes = (0..size)
            .map(|atlas_index| UnusedNode {
                node_id: INVALID_NODE_ID,
                atlas_index,
            })
            .collect();

        Self {
            load_events: default(),
            loaded_nodes: default(),
            loading_nodes: default(),
            nodes: default(),
            data: vec![default(); size as usize],
            attachments,
            size,
            unused_nodes,
            existing_nodes,
        }
    }

    /// Creates a new quadtree from a terrain config.
    pub fn from_config(config: &TerrainConfig) -> Self {
        Self::new(
            config.node_atlas_size as u16,
            config.attachments.clone(),
            config.nodes.clone(),
        )
    }

    /// Adjusts the node atlas according to the requested and released nodes of the [`Quadtree`]
    /// and starts loading not already present nodes.
    fn fulfill_request(&mut self, quadtree: &mut Quadtree) {
        let NodeAtlas {
            attachments,
            unused_nodes,
            nodes,
            loading_nodes,
            load_events,
            existing_nodes,
            ..
        } = self;

        // release nodes that are on longer required
        for node_id in quadtree.released_nodes.drain(..) {
            if !existing_nodes.contains(&node_id) {
                continue;
            }

            let node = nodes
                .get_mut(&node_id)
                .expect("Tried releasing a node, which is not present.");
            node.requests -= 1;

            if node.requests == 0 {
                // the node is not used anymore
                unused_nodes.push_back(UnusedNode {
                    node_id,
                    atlas_index: node.atlas_index,
                });
            }
        }

        // load nodes that are requested
        for node_id in quadtree.requested_nodes.drain(..) {
            if !existing_nodes.contains(&node_id) {
                continue;
            }

            // check if the node is already present else start loading it
            if let Some(node) = nodes.get_mut(&node_id) {
                if node.requests == 0 {
                    // the node is now used again
                    unused_nodes.retain(|unused_node| node.atlas_index != unused_node.atlas_index);
                }

                node.requests += 1;
            } else {
                // Todo: implement better loading strategy
                // remove least recently used node and reuse its atlas index
                let unused_node = unused_nodes.pop_front().expect("Atlas out of indices");

                nodes.remove(&unused_node.node_id);
                nodes.insert(
                    node_id,
                    AtlasNode {
                        requests: 1,
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
                        loading_attachments: (0..attachments.len()).collect(),
                        attachments: default(),
                    },
                );
            }
        }

        // println!(
        //     "Currently there are {} nodes in use.",
        //     self.size as usize - self.unused_nodes.len()
        // );
    }

    /// Checks all nodes that have finished loading, marks them accordingly and prepares the data
    /// to be send to the gpu by the [`GpuNodeAtlas`](super::gpu_node_atlas::GpuNodeAtlas).
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
        for (node_id, loading_node) in loading_nodes.extract_if(|_, node| node.finished_loading())
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
