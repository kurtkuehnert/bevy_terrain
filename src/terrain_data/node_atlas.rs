use crate::preprocess::R16Image;
use crate::{
    terrain::{Terrain, TerrainConfig},
    terrain_data::{quadtree::Quadtree, AtlasAttachment, NodeCoordinate},
    terrain_view::{TerrainView, TerrainViewComponents},
};
use bevy::render::render_resource::ShaderType;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use image::io::Reader;
use itertools::Itertools;
use std::collections::VecDeque;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

/// Stores all of the attachments of the node, alongside their loading state.
// #[derive(Clone)]
// pub struct LoadingNode {
//     /// The atlas index of the node.
//     pub(crate) atlas_index: u32,
//     // Todo: replace with array or vec of options
//     /// Stores all of the nodes attachments.
//     pub(crate) attachments: HashMap<AttachmentIndex, Handle<Image>>,
//     /// The set of still loading attachments. Is empty if the node is fully loaded.
//     loading_attachments: HashSet<AttachmentIndex>,
// }
//
// impl LoadingNode {
//     /// Sets the attachment data of the node.
//     pub fn set_attachment(&mut self, attachment_index: AttachmentIndex, attachment: Handle<Image>) {
//         self.attachments.insert(attachment_index, attachment);
//     }
//
//     /// Marks the corresponding attachment as loaded.
//     pub fn loaded(&mut self, attachment_index: AttachmentIndex) {
//         self.loading_attachments.remove(&attachment_index);
//     }
//
//     /// Returns whether all node attachments of the node have finished loading.
//     fn finished_loading(&self) -> bool {
//         self.loading_attachments.is_empty()
//     }
// }

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
    pub(crate) atlas_index: u32,
    /// The count of [`Quadtrees`] that have requested this node.
    requests: u32,
}

#[derive(Copy, Clone, Debug, Default, ShaderType)]
pub(crate) struct NodeMeta {
    pub(crate) node_coordinate: NodeCoordinate,
    #[size(16)]
    pub(crate) atlas_index: u32,
}

pub(crate) fn format_node_path(
    path: &str,
    node_coordinate: &NodeCoordinate,
    extension: &str,
) -> String {
    format!("{path}/{node_coordinate}.{extension}",)
}

#[derive(Clone, Debug)]
pub(crate) struct NodeWithData {
    pub(crate) meta: NodeMeta,
    pub(crate) data: Vec<u16>,
    pub(crate) texture_size: u32,
}

impl NodeWithData {
    pub(crate) fn start_saving(self, path: String) -> Task<NodeMeta> {
        AsyncComputeTaskPool::get().spawn(async move {
            let path = format_node_path(&path, &self.meta.node_coordinate, "png");

            let image =
                R16Image::from_raw(self.texture_size, self.texture_size, self.data).unwrap();

            image.save(&path).unwrap();

            println!("Finished saving node: {path}");

            self.meta
        })
    }

    pub(crate) fn start_loading(node: NodeMeta, path: String) -> Task<Self> {
        AsyncComputeTaskPool::get().spawn(async move {
            let path = format_node_path(&path, &node.node_coordinate, "png");

            let mut reader = Reader::open(path).unwrap();
            reader.no_limits();
            let image = reader.decode().unwrap().into_luma16();
            let texture_size = image.width();
            let data = image.into_raw();

            Self {
                meta: node,
                data,
                texture_size,
            }
        })
    }
}

/// A sparse storage of all terrain attachments, which streams data in and out of memory
/// depending on the decisions of the corresponding [`Quadtree`]s.
///
/// A node is considered present and assigned an [`u32`] as soon as it is
/// requested by any quadtree. Then the node atlas will start loading all of its attachments
/// by storing the [`NodeCoordinate`] (for one frame) in `load_events` for which
/// attachment-loading-systems can listen.
/// Nodes that are not being used by any quadtree anymore are cached (LRU),
/// until new atlas indices are required.
///
/// The [`u32`] can be used for accessing the attached data in systems by the CPU
/// and in shaders by the GPU.
#[derive(Component)]
pub struct NodeAtlas {
    pub(crate) attachments: Vec<AtlasAttachment>, // stores the attachment data
    pub(crate) nodes: HashMap<NodeCoordinate, AtlasNode>,

    // pub(crate) start_loading_nodes: Vec<NodeCoordinate>,
    pub(crate) finished_loading_nodes: Vec<NodeWithData>,

    pub(crate) loading_nodes: Vec<Task<NodeWithData>>,

    pub(crate) read_back_nodes: Arc<Mutex<Vec<Task<Vec<NodeWithData>>>>>,
    pub(crate) saving_nodes: Vec<Task<NodeMeta>>,

    unused_nodes: VecDeque<NodeMeta>,

    pub(crate) existing_nodes: HashSet<NodeCoordinate>,
    pub(crate) size: u32,

    pub(crate) slots: u32,
    pub(crate) max_slots: u32,

    path: String,
}

impl NodeAtlas {
    /// Creates a new quadtree from parameters.
    ///
    /// * `size` - The amount of nodes the can be loaded simultaneously in the node atlas.
    /// * `attachments` - The atlas attachments of the terrain.
    pub fn new(
        size: u32,
        attachments: Vec<AtlasAttachment>,
        existing_nodes: HashSet<NodeCoordinate>,
    ) -> Self {
        let unused_nodes = (0..size)
            .map(|atlas_index| NodeMeta {
                node_coordinate: NodeCoordinate::INVALID,
                atlas_index,
            })
            .collect();

        let path = "terrains/basic";
        let path = format!("assets/{path}/data/height");

        Self {
            finished_loading_nodes: default(),
            loading_nodes: default(),
            nodes: default(),
            attachments,
            size,
            unused_nodes,
            existing_nodes,

            read_back_nodes: default(),
            saving_nodes: default(),
            slots: 16,
            max_slots: 16,
            // start_loading_nodes: vec![],
            path,
        }
    }

    /// Creates a new quadtree from a terrain config.
    pub fn from_config(config: &TerrainConfig) -> Self {
        Self::new(
            config.node_atlas_size,
            config
                .attachments
                .clone()
                .into_iter()
                .map(|attachment| attachment.into())
                .collect_vec(),
            config.nodes.clone(),
        )
    }

    fn reserve_atlas_index(&mut self) -> u32 {
        let unused_node = self.unused_nodes.pop_front().expect("Atlas out of indices");

        self.nodes.remove(&unused_node.node_coordinate);

        unused_node.atlas_index
    }

    pub fn get_or_allocate(&mut self, node_coordinate: NodeCoordinate) -> u32 {
        if let Some(node) = self.nodes.get(&node_coordinate) {
            node.atlas_index
        } else {
            let atlas_index = self.reserve_atlas_index();

            self.nodes.insert(
                node_coordinate,
                AtlasNode {
                    requests: 1,
                    state: LoadingState::Loaded,
                    atlas_index,
                },
            );

            self.loading_nodes.push(NodeWithData::start_loading(
                NodeMeta {
                    node_coordinate,
                    atlas_index,
                },
                self.path.clone(),
            ));

            atlas_index
        }
    }

    fn update(&mut self) {
        let NodeAtlas {
            read_back_nodes,
            saving_nodes,
            loading_nodes,
            finished_loading_nodes,
            nodes,
            slots,
            ..
        } = self;

        loading_nodes.retain_mut(|node| {
            if let Some(node) = future::block_on(future::poll_once(node)) {
                nodes.get_mut(&node.meta.node_coordinate).unwrap().state = LoadingState::Loaded;
                finished_loading_nodes.push(node);
                false
            } else {
                true
            }
        });

        read_back_nodes
            .lock()
            .unwrap()
            .deref_mut()
            .retain_mut(|nodes| {
                if let Some(nodes) = future::block_on(future::poll_once(nodes)) {
                    for node in nodes {
                        saving_nodes.push(node.start_saving(self.path.clone()));
                    }
                    false
                } else {
                    true
                }
            });

        saving_nodes.retain_mut(|task| {
            if future::block_on(future::poll_once(task)).is_some() {
                *slots += 1;
                false
            } else {
                true
            }
        });
    }

    /// Adjusts the node atlas according to the requested and released nodes of the [`Quadtree`]
    /// and starts loading not already present nodes.
    fn fulfill_request(&mut self, quadtree: &mut Quadtree) {
        let NodeAtlas {
            unused_nodes,
            nodes,
            loading_nodes,
            existing_nodes,
            ..
        } = self;

        // release nodes that are on longer required
        for node_coordinate in quadtree.released_nodes.drain(..) {
            if !existing_nodes.contains(&node_coordinate) {
                continue;
            }

            let node = nodes
                .get_mut(&node_coordinate)
                .expect("Tried releasing a node, which is not present.");
            node.requests -= 1;

            if node.requests == 0 {
                // the node is not used anymore
                unused_nodes.push_back(NodeMeta {
                    node_coordinate,
                    atlas_index: node.atlas_index,
                });
            }
        }

        // load nodes that are requested
        for node_coordinate in quadtree.requested_nodes.drain(..) {
            if !existing_nodes.contains(&node_coordinate) {
                continue;
            }

            // check if the node is already present else start loading it
            if let Some(node) = nodes.get_mut(&node_coordinate) {
                if node.requests == 0 {
                    // the node is now used again
                    unused_nodes.retain(|unused_node| node.atlas_index != unused_node.atlas_index);
                }

                node.requests += 1;
            } else {
                // Todo: implement better loading strategy
                // remove least recently used node and reuse its atlas index
                let unused_node = unused_nodes.pop_front().expect("Atlas out of indices");
                nodes.remove(&unused_node.node_coordinate);
                let atlas_index = unused_node.atlas_index;

                nodes.insert(
                    node_coordinate,
                    AtlasNode {
                        requests: 1,
                        state: LoadingState::Loading,
                        atlas_index,
                    },
                );

                loading_nodes.push(NodeWithData::start_loading(
                    NodeMeta {
                        node_coordinate,
                        atlas_index,
                    },
                    self.path.clone(),
                ));
            }
        }

        // println!(
        //     "Currently there are {} nodes in use.",
        //     self.size as usize - self.unused_nodes.len()
        // );
    }
}

/// Updates the node atlas according to all corresponding quadtrees.
pub(crate) fn update_node_atlas(
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    mut terrain_query: Query<(Entity, &mut NodeAtlas), With<Terrain>>,
) {
    for (terrain, mut node_atlas) in terrain_query.iter_mut() {
        node_atlas.update();

        for view in view_query.iter() {
            if let Some(quadtree) = quadtrees.get_mut(&(terrain, view)) {
                node_atlas.fulfill_request(quadtree);
            }
        }
    }
}
