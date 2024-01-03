use crate::formats::tc::load_node_config;
use crate::terrain_data::quadtree::NodeLookup;
use crate::{
    prelude::{AttachmentConfig, AttachmentFormat},
    preprocess::{R16Image, Rg16Image, Rgba8Image},
    terrain::{Terrain, TerrainConfig},
    terrain_data::{
        quadtree::{Quadtree, QuadtreeEntry},
        AttachmentData, NodeCoordinate, INVALID_ATLAS_INDEX, INVALID_LOD,
    },
    terrain_view::{TerrainView, TerrainViewComponents},
};
use bevy::{
    prelude::*,
    render::render_resource::*,
    tasks::{futures_lite::future, AsyncComputeTaskPool, Task},
    utils::{HashMap, HashSet},
};
use image::{io::Reader, DynamicImage};
use itertools::Itertools;
use std::{collections::VecDeque, mem};

#[derive(Copy, Clone, Debug, Default, ShaderType)]
pub(crate) struct AtlasNode {
    pub(crate) coordinate: NodeCoordinate,
    #[size(16)]
    pub(crate) atlas_index: u32,
}

#[derive(Clone)]
pub(crate) struct NodeWithData {
    pub(crate) node: AtlasNode,
    pub(crate) data: AttachmentData,
    pub(crate) texture_size: u32,
}

impl NodeWithData {
    pub(crate) fn start_saving(self, path: String) -> Task<AtlasNode> {
        AsyncComputeTaskPool::get().spawn(async move {
            let path = self.node.coordinate.path(&path, "png");

            dbg!(&path);

            let image = match self.data {
                AttachmentData::Rgba8(data) => {
                    let data = data.into_iter().flatten().collect_vec();
                    DynamicImage::from(
                        Rgba8Image::from_raw(self.texture_size, self.texture_size, data).unwrap(),
                    )
                }
                AttachmentData::R16(data) => DynamicImage::from(
                    R16Image::from_raw(self.texture_size, self.texture_size, data).unwrap(),
                ),
                AttachmentData::Rg16(data) => {
                    let data = data.into_iter().flatten().collect_vec();
                    DynamicImage::from(
                        Rg16Image::from_raw(self.texture_size, self.texture_size, data).unwrap(),
                    )
                }
                AttachmentData::None => panic!("Attachment has not data."),
            };

            image.save(&path).unwrap();

            println!("Finished saving node: {path}");

            self.node
        })
    }

    pub(crate) fn start_loading(
        node: AtlasNode,
        path: String,
        format: AttachmentFormat,
    ) -> Task<Self> {
        AsyncComputeTaskPool::get().spawn(async move {
            let path = node.coordinate.path(&path, "png");

            let mut reader = Reader::open(path).unwrap();
            reader.no_limits();
            let image = reader.decode().unwrap();
            let texture_size = image.width();
            let data = AttachmentData::from_bytes(image.as_bytes(), format);

            Self {
                node,
                data,
                texture_size,
            }
        })
    }
}

/// An attachment of a [`NodeAtlas`].
pub struct AtlasAttachment {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) texture_size: u32,
    pub(crate) center_size: u32,
    pub(crate) border_size: u32,
    scale: f32,
    offset: f32,
    pub(crate) mip_level_count: u32,
    pub(crate) format: AttachmentFormat,
    pub(crate) data: Vec<AttachmentData>,

    pub(crate) saving_nodes: Vec<Task<AtlasNode>>,
    pub(crate) loading_nodes: Vec<Task<NodeWithData>>,

    pub(crate) upload_nodes: Vec<NodeWithData>,
    pub(crate) download_nodes: Vec<Task<NodeWithData>>,
}

impl AtlasAttachment {
    fn new(config: &AttachmentConfig, node_atlas_size: u32, path: &str) -> Self {
        let name = config.name.clone();
        let path = format!("assets/{path}/data/{name}");

        Self {
            name,
            path,
            texture_size: config.texture_size,
            center_size: config.center_size,
            border_size: config.border_size,
            scale: config.center_size as f32 / config.texture_size as f32,
            offset: config.border_size as f32 / config.texture_size as f32,
            mip_level_count: config.mip_level_count,
            format: config.format,
            data: vec![AttachmentData::None; node_atlas_size as usize],
            upload_nodes: default(),
            loading_nodes: default(),
            saving_nodes: default(),
            download_nodes: default(),
        }
    }

    fn add_node_data(&mut self, data: AttachmentData, atlas_index: u32) {
        self.data[atlas_index as usize] = data;
    }

    fn update(&mut self, atlas_state: &mut NodeAtlasState) {
        // Todo: build customizable loader abstraction
        for &node in &atlas_state.start_loading_nodes {
            self.loading_nodes.push(NodeWithData::start_loading(
                node,
                self.path.clone(),
                self.format,
            ));
        }

        let mut loading_nodes = mem::take(&mut self.loading_nodes);
        loading_nodes.retain_mut(|node| {
            if let Some(node) = future::block_on(future::poll_once(node)) {
                self.add_node_data(node.data.clone(), node.node.atlas_index);
                atlas_state.loaded_node_attachment(node.node.coordinate, 0);
                self.upload_nodes.push(node);
                false
            } else {
                true
            }
        });
        self.loading_nodes = loading_nodes;

        self.download_nodes.retain_mut(|nodes| {
            if let Some(node) = future::block_on(future::poll_once(nodes)) {
                self.saving_nodes.push(node.start_saving(self.path.clone()));
                false
            } else {
                true
            }
        });

        self.saving_nodes.retain_mut(|task| {
            if future::block_on(future::poll_once(task)).is_some() {
                atlas_state.slots += 1;
                false
            } else {
                true
            }
        });
    }

    fn sample(&self, lookup: NodeLookup) -> Vec4 {
        if lookup.atlas_index == INVALID_ATLAS_INDEX {
            return Vec4::splat(0.0); // Todo: Handle this better
        }

        let data = &self.data[lookup.atlas_index as usize];

        let coordinate = lookup.atlas_coordinate * self.scale + self.offset;

        return data.sample(coordinate, self.texture_size);
    }
}

/// The current state of a node of a [`NodeAtlas`].
///
/// This indicates, whether the node is loading or loaded and ready to be used.
#[derive(Clone, Copy)]
enum LoadingState {
    /// The node is loading, but can not be used yet.
    Loading(u32),
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
    unused_nodes: VecDeque<AtlasNode>,
    existing_nodes: HashSet<NodeCoordinate>,

    attachment_count: u32,

    start_loading_nodes: Vec<AtlasNode>,

    pub(crate) slots: u32,
    pub(crate) max_slots: u32,
}

impl NodeAtlasState {
    fn new(
        atlas_size: u32,
        attachment_count: u32,
        existing_nodes: HashSet<NodeCoordinate>,
    ) -> Self {
        let unused_nodes = (0..atlas_size)
            .map(|atlas_index| AtlasNode {
                coordinate: NodeCoordinate::INVALID,
                atlas_index,
            })
            .collect();

        Self {
            node_states: default(),
            unused_nodes,
            existing_nodes,
            attachment_count,
            start_loading_nodes: default(),
            slots: 16,
            max_slots: 16,
        }
    }
    fn loaded_node_attachment(&mut self, node_coordinate: NodeCoordinate, _attachment_index: u32) {
        let node_state = self.node_states.get_mut(&node_coordinate).unwrap();

        node_state.state = match node_state.state {
            LoadingState::Loading(1) => LoadingState::Loaded,
            LoadingState::Loading(n) => LoadingState::Loading(n - 1),
            LoadingState::Loaded => {
                panic!("Loaded more attachments, than registered with the atlas.")
            }
        };
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
                    state: LoadingState::Loading(self.attachment_count),
                    atlas_index,
                },
            );

            self.start_loading_nodes.push(AtlasNode {
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
            self.unused_nodes.push_back(AtlasNode {
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
    pub(crate) lod_count: u32,
}

impl NodeAtlas {
    /// Creates a new quadtree from parameters.
    ///
    /// * `size` - The amount of nodes the can be loaded simultaneously in the node atlas.
    /// * `attachments` - The atlas attachments of the terrain.
    pub fn new(
        path: &str,
        atlas_size: u32,
        lod_count: u32,
        attachments: &[AttachmentConfig],
    ) -> Self {
        let attachments = attachments
            .iter()
            .map(|attachment| AtlasAttachment::new(attachment, atlas_size, path))
            .collect_vec();

        let existing_nodes = load_node_config(path);

        let state = NodeAtlasState::new(atlas_size, attachments.len() as u32, existing_nodes);

        Self {
            attachments,
            state,
            atlas_size,
            lod_count,
        }
    }

    /// Creates a new quadtree from a terrain config.
    pub fn from_config(config: &TerrainConfig) -> Self {
        Self::new(
            &config.path,
            config.node_atlas_size,
            config.lod_count,
            &config.attachments,
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

    pub(crate) fn sample_attachment(&self, node_lookup: NodeLookup, attachment_index: u32) -> Vec4 {
        self.attachments[attachment_index as usize].sample(node_lookup)
    }

    fn update(&mut self) {
        let NodeAtlas {
            state, attachments, ..
        } = self;

        for attachment in attachments {
            attachment.update(state);
        }

        state.start_loading_nodes.clear();
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
