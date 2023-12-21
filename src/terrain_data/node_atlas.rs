use crate::{
    prelude::{AttachmentConfig, AttachmentFormat},
    preprocess::R16Image,
    terrain::{Terrain, TerrainConfig},
    terrain_data::{
        quadtree::{Quadtree, QuadtreeEntry},
        NodeCoordinate, INVALID_ATLAS_INDEX, INVALID_LOD,
    },
    terrain_view::{TerrainView, TerrainViewComponents},
};
use bevy::{
    prelude::*,
    render::render_resource::ShaderType,
    tasks::{futures_lite::future, AsyncComputeTaskPool, Task},
    utils::{HashMap, HashSet},
};
use image::io::Reader;
use itertools::Itertools;
use std::{
    collections::VecDeque,
    mem,
    ops::DerefMut,
    sync::{Arc, Mutex},
};

// Todo: find better name (AtlasNode?)
#[derive(Copy, Clone, Debug, Default, ShaderType)]
pub(crate) struct NodeMeta {
    pub(crate) coordinate: NodeCoordinate,
    #[size(16)]
    pub(crate) atlas_index: u32,
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
            let path = self.meta.coordinate.path(&path, "png");

            let image =
                R16Image::from_raw(self.texture_size, self.texture_size, self.data).unwrap();

            image.save(&path).unwrap();

            println!("Finished saving node: {path}");

            self.meta
        })
    }

    pub(crate) fn start_loading(node: NodeMeta, path: String) -> Task<Self> {
        AsyncComputeTaskPool::get().spawn(async move {
            let path = node.coordinate.path(&path, "png");

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

type AttachmentData = Vec<u16>;

/// An attachment of a [`NodeAtlas`].
pub struct AtlasAttachment {
    /// The name of the attachment.
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) texture_size: u32,
    pub(crate) center_size: u32,
    pub(crate) border_size: u32,
    pub(crate) mip_level_count: u32,
    /// The format of the attachment.
    pub(crate) format: AttachmentFormat,
    pub(crate) data: Vec<AttachmentData>,

    pub(crate) saving_nodes: Vec<Task<NodeMeta>>,
    pub(crate) loading_nodes: Vec<Task<NodeWithData>>,

    pub(crate) finished_loading_nodes: Vec<NodeWithData>,
    pub(crate) read_back_nodes: Arc<Mutex<Vec<Task<Vec<NodeWithData>>>>>,
}

impl AtlasAttachment {
    fn new(config: &AttachmentConfig, node_atlas_size: u32) -> Self {
        let path = format!("assets/terrains/basic/data/height");

        Self {
            name: config.name.clone(),
            path,
            texture_size: config.texture_size,
            center_size: config.texture_size - 2 * config.border_size,
            border_size: config.border_size,
            mip_level_count: config.mip_level_count,
            format: config.format,
            data: vec![Vec::new(); node_atlas_size as usize],
            finished_loading_nodes: default(),
            loading_nodes: default(),
            saving_nodes: default(),
            read_back_nodes: default(),
        }
    }

    fn add_node_data(&mut self, data: AttachmentData, atlas_index: u32) {
        self.data[atlas_index as usize] = data;
    }

    fn update(&mut self, atlas_state: &mut NodeAtlasState) {
        let mut loading_nodes = mem::take(&mut self.loading_nodes);
        loading_nodes.retain_mut(|node| {
            if let Some(node) = future::block_on(future::poll_once(node)) {
                self.add_node_data(node.data.clone(), node.meta.atlas_index);
                atlas_state.loaded_node_attachment(node.meta.coordinate, 0);
                self.finished_loading_nodes.push(node);
                false
            } else {
                true
            }
        });
        self.loading_nodes = loading_nodes;

        self.read_back_nodes
            .lock()
            .unwrap()
            .deref_mut()
            .retain_mut(|nodes| {
                if let Some(nodes) = future::block_on(future::poll_once(nodes)) {
                    for node in nodes {
                        self.saving_nodes.push(node.start_saving(self.path.clone()));
                    }
                    false
                } else {
                    true
                }
            });

        let mut saving_nodes = mem::take(&mut self.saving_nodes);
        saving_nodes.retain_mut(|task| {
            if future::block_on(future::poll_once(task)).is_some() {
                atlas_state.slots += 1;
                false
            } else {
                true
            }
        });
        self.saving_nodes = saving_nodes;
    }

    // Todo: Implement get data at coordinate function
}

/// The current state of a node of a [`NodeAtlas`].
///
/// This indicates, whether the node is loading or loaded and ready to be used.
#[derive(Clone, Copy)]
enum LoadingState {
    /// The node is loading, but can not be used yet.
    Loading,
    /// The node is loaded and can be used.
    Loaded,
}

/// The internal representation of a present node in a [`NodeAtlas`].
struct NodeState {
    /// Indicates whether or not the node is loading or loaded.
    state: LoadingState,
    /// The index of the node inside the atlas.
    atlas_index: u32,
    /// The count of [`Quadtrees`] that have requested this node.
    requests: u32,
}

pub(crate) struct NodeAtlasState {
    node_states: HashMap<NodeCoordinate, NodeState>,
    unused_nodes: VecDeque<NodeMeta>,
    existing_nodes: HashSet<NodeCoordinate>,

    start_loading_nodes: Vec<NodeMeta>,

    pub(crate) slots: u32,
    pub(crate) max_slots: u32,
}

impl NodeAtlasState {
    fn new(atlas_size: u32, existing_nodes: HashSet<NodeCoordinate>) -> Self {
        let unused_nodes = (0..atlas_size)
            .map(|atlas_index| NodeMeta {
                coordinate: NodeCoordinate::INVALID,
                atlas_index,
            })
            .collect();

        Self {
            node_states: default(),
            unused_nodes,
            existing_nodes,
            start_loading_nodes: default(),
            slots: 16,
            max_slots: 16,
        }
    }
    fn loaded_node_attachment(&mut self, node_coordinate: NodeCoordinate, attachment_index: u32) {
        let node_state = self.node_states.get_mut(&node_coordinate).unwrap();
        node_state.state = LoadingState::Loaded;
    }

    fn allocate_node(&mut self) -> u32 {
        let unused_node = self.unused_nodes.pop_front().expect("Atlas out of indices");

        self.node_states.remove(&unused_node.coordinate);

        unused_node.atlas_index
    }

    fn get_or_allocate(&mut self, node_coordinate: NodeCoordinate) -> u32 {
        if let Some(node) = self.node_states.get(&node_coordinate) {
            node.atlas_index
        } else {
            let atlas_index = self.allocate_node();

            self.node_states.insert(
                node_coordinate,
                NodeState {
                    requests: 1,
                    state: LoadingState::Loaded,
                    atlas_index,
                },
            );

            atlas_index
        }
    }

    fn request_node(&mut self, node_coordinate: NodeCoordinate) {
        if !self.existing_nodes.contains(&node_coordinate) {
            return;
        }

        let mut node_states = mem::take(&mut self.node_states);

        // check if the node is already present else start loading it
        if let Some(node) = node_states.get_mut(&node_coordinate) {
            if node.requests == 0 {
                // the node is now used again
                self.unused_nodes
                    .retain(|unused_node| node.atlas_index != unused_node.atlas_index);
            }

            node.requests += 1;
        } else {
            // Todo: implement better loading strategy
            let atlas_index = self.allocate_node();

            node_states.insert(
                node_coordinate,
                NodeState {
                    requests: 1,
                    state: LoadingState::Loading,
                    atlas_index,
                },
            );

            self.start_loading_nodes.push(NodeMeta {
                coordinate: node_coordinate,
                atlas_index,
            });
        }

        self.node_states = node_states;
    }

    fn release_node(&mut self, node_coordinate: NodeCoordinate) {
        if !self.existing_nodes.contains(&node_coordinate) {
            return;
        }

        let node = self
            .node_states
            .get_mut(&node_coordinate)
            .expect("Tried releasing a node, which is not present.");
        node.requests -= 1;

        if node.requests == 0 {
            // the node is not used anymore
            self.unused_nodes.push_back(NodeMeta {
                coordinate: node_coordinate,
                atlas_index: node.atlas_index,
            });
        }
    }

    fn get_best_node(&self, node_coordinate: NodeCoordinate, lod_count: u32) -> QuadtreeEntry {
        let mut best_node_coordinate = node_coordinate;

        loop {
            if best_node_coordinate == NodeCoordinate::INVALID
                || best_node_coordinate.lod == lod_count
            {
                // highest lod is not loaded
                return QuadtreeEntry {
                    atlas_index: INVALID_ATLAS_INDEX,
                    atlas_lod: INVALID_LOD,
                };
            }

            if let Some(atlas_node) = self.node_states.get(&best_node_coordinate) {
                if matches!(atlas_node.state, LoadingState::Loaded) {
                    // found best loaded node
                    return QuadtreeEntry {
                        atlas_index: atlas_node.atlas_index,
                        atlas_lod: best_node_coordinate.lod,
                    };
                }
            }

            best_node_coordinate = best_node_coordinate.parent();
        }
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
    pub(crate) state: NodeAtlasState,
    pub(crate) atlas_size: u32,
}

impl NodeAtlas {
    /// Creates a new quadtree from parameters.
    ///
    /// * `size` - The amount of nodes the can be loaded simultaneously in the node atlas.
    /// * `attachments` - The atlas attachments of the terrain.
    pub fn new(
        size: u32,
        attachments: &Vec<AttachmentConfig>,
        existing_nodes: HashSet<NodeCoordinate>,
    ) -> Self {
        let attachments = attachments
            .iter()
            .map(|attachment| AtlasAttachment::new(attachment, size))
            .collect_vec();

        let state = NodeAtlasState::new(size, existing_nodes);

        Self {
            attachments,
            atlas_size: size,
            state,
        }
    }

    /// Creates a new quadtree from a terrain config.
    pub fn from_config(config: &TerrainConfig) -> Self {
        Self::new(
            config.node_atlas_size,
            &config.attachments,
            config.nodes.clone(),
        )
    }

    pub fn get_or_allocate(&mut self, node_coordinate: NodeCoordinate) -> u32 {
        self.state.get_or_allocate(node_coordinate)
    }

    pub(crate) fn get_best_node(
        &self,
        node_coordinate: NodeCoordinate,
        lod_count: u32,
    ) -> QuadtreeEntry {
        self.state.get_best_node(node_coordinate, lod_count)
    }

    fn update(&mut self) {
        let NodeAtlas {
            state, attachments, ..
        } = self;

        let attachment = &mut attachments[0];

        for node in state.start_loading_nodes.drain(..) {
            attachment
                .loading_nodes
                .push(NodeWithData::start_loading(node, attachment.path.clone()));
        }

        attachment.update(state);
    }

    /// Adjusts the node atlas according to the requested and released nodes of the [`Quadtree`]
    /// and starts loading not already present nodes.
    fn fulfill_request(&mut self, quadtree: &mut Quadtree) {
        for node_coordinate in quadtree.released_nodes.drain(..) {
            self.state.release_node(node_coordinate);
        }

        for node_coordinate in quadtree.requested_nodes.drain(..) {
            self.state.request_node(node_coordinate);
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
