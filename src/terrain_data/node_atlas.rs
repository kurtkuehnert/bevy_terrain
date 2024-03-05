use crate::{
    formats::TC,
    prelude::{AttachmentConfig, AttachmentFormat},
    terrain::{Terrain, TerrainConfig},
    terrain_data::{
        coordinates::NodeCoordinate,
        quadtree::{NodeLookup, Quadtree, QuadtreeEntry},
        AttachmentData, INVALID_ATLAS_INDEX, INVALID_LOD,
    },
    terrain_view::{TerrainView, TerrainViewComponents},
};
use anyhow::Result;
use bevy::{
    prelude::*,
    render::render_resource::*,
    tasks::{futures_lite::future, AsyncComputeTaskPool, Task},
    utils::{HashMap, HashSet},
};
use bit_set::BitSet;
use image::{io::Reader, DynamicImage, ImageBuffer, Luma, LumaA, Rgb, Rgba};
use itertools::Itertools;
use std::{collections::VecDeque, fs, mem, ops::DerefMut};

pub type Rgb8Image = ImageBuffer<Rgb<u8>, Vec<u8>>;
pub type Rgba8Image = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type R16Image = ImageBuffer<Luma<u16>, Vec<u16>>;
pub type Rg16Image = ImageBuffer<LumaA<u16>, Vec<u16>>;

const STORE_PNG: bool = false;

#[derive(Copy, Clone, Debug, Default, ShaderType)]
pub struct AtlasNode {
    pub(crate) coordinate: NodeCoordinate,
    #[size(16)]
    pub(crate) atlas_index: u32,
}

impl AtlasNode {
    pub fn new(node_coordinate: NodeCoordinate, atlas_index: u32) -> Self {
        Self {
            coordinate: node_coordinate,
            atlas_index,
        }
    }
    pub fn attachment(self, attachment_index: u32) -> AtlasNodeAttachment {
        AtlasNodeAttachment {
            coordinate: self.coordinate,
            atlas_index: self.atlas_index,
            attachment_index,
        }
    }
}

impl From<AtlasNodeAttachment> for AtlasNode {
    fn from(node: AtlasNodeAttachment) -> Self {
        Self {
            coordinate: node.coordinate,
            atlas_index: node.atlas_index,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct AtlasNodeAttachment {
    pub(crate) coordinate: NodeCoordinate,
    pub(crate) atlas_index: u32,
    pub(crate) attachment_index: u32,
}

#[derive(Clone)]
pub(crate) struct NodeAttachmentWithData {
    pub(crate) node: AtlasNodeAttachment,
    pub(crate) data: AttachmentData,
    pub(crate) texture_size: u32,
}

impl NodeAttachmentWithData {
    pub(crate) fn start_saving(self, path: String) -> Task<AtlasNodeAttachment> {
        AsyncComputeTaskPool::get().spawn(async move {
            if STORE_PNG {
                let path = self.node.coordinate.path(&path, "png");

                let image = match self.data {
                    AttachmentData::Rgba8(data) => {
                        let data = data.into_iter().flatten().collect_vec();
                        DynamicImage::from(
                            Rgba8Image::from_raw(self.texture_size, self.texture_size, data)
                                .unwrap(),
                        )
                    }
                    AttachmentData::R16(data) => DynamicImage::from(
                        R16Image::from_raw(self.texture_size, self.texture_size, data).unwrap(),
                    ),
                    AttachmentData::Rg16(data) => {
                        let data = data.into_iter().flatten().collect_vec();
                        DynamicImage::from(
                            Rg16Image::from_raw(self.texture_size, self.texture_size, data)
                                .unwrap(),
                        )
                    }
                    AttachmentData::None => panic!("Attachment has not data."),
                };

                image.save(&path).unwrap();

                println!("Finished saving node: {path}");
            } else {
                let path = self.node.coordinate.path(&path, "bin");

                fs::write(path, self.data.bytes()).unwrap();

                // println!("Finished saving node: {path}");
            }

            self.node
        })
    }

    pub(crate) fn start_loading(
        node: AtlasNodeAttachment,
        path: String,
        texture_size: u32,
        format: AttachmentFormat,
        mip_level_count: u32,
    ) -> Task<Result<Self>> {
        AsyncComputeTaskPool::get().spawn(async move {
            let mut data = if STORE_PNG {
                let path = node.coordinate.path(&path, "png");

                let mut reader = Reader::open(path)?;
                reader.no_limits();
                let image = reader.decode().unwrap();
                AttachmentData::from_bytes(image.as_bytes(), format)
            } else {
                let path = node.coordinate.path(&path, "bin");

                let bytes = fs::read(path)?;

                AttachmentData::from_bytes(&bytes, format)
            };

            data.generate_mipmaps(texture_size, mip_level_count);

            Ok(Self {
                node,
                data,
                texture_size: 0,
            })
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

    pub(crate) saving_nodes: Vec<Task<AtlasNodeAttachment>>,
    pub(crate) loading_nodes: Vec<Task<Result<NodeAttachmentWithData>>>,
    pub(crate) uploading_nodes: Vec<NodeAttachmentWithData>,
    pub(crate) downloading_nodes: Vec<Task<NodeAttachmentWithData>>,
}

impl AtlasAttachment {
    fn new(config: &AttachmentConfig, node_atlas_size: u32, path: &str) -> Self {
        let name = config.name.clone();
        let path = format!("assets/{path}/data/{name}");
        let center_size = config.texture_size - 2 * config.border_size;

        Self {
            name,
            path,
            texture_size: config.texture_size,
            center_size,
            border_size: config.border_size,
            scale: center_size as f32 / config.texture_size as f32,
            offset: config.border_size as f32 / config.texture_size as f32,
            mip_level_count: config.mip_level_count,
            format: config.format,
            data: vec![AttachmentData::None; node_atlas_size as usize],
            saving_nodes: default(),
            loading_nodes: default(),
            uploading_nodes: default(),
            downloading_nodes: default(),
        }
    }

    fn update(&mut self, atlas_state: &mut NodeAtlasState) {
        self.loading_nodes.retain_mut(|node| {
            future::block_on(future::poll_once(node)).map_or(true, |node| {
                if let Ok(node) = node {
                    atlas_state.loaded_node_attachment(node.node);
                    self.uploading_nodes.push(node.clone());
                    self.data[node.node.atlas_index as usize] = node.data;
                } else {
                    dbg!("loading failed");
                    atlas_state.load_slots += 1;
                }

                false
            })
        });

        self.downloading_nodes.retain_mut(|node| {
            future::block_on(future::poll_once(node)).map_or(true, |node| {
                atlas_state.downloaded_node_attachment(node.node);
                self.data[node.node.atlas_index as usize] = node.data;
                false
            })
        });

        self.saving_nodes.retain_mut(|task| {
            future::block_on(future::poll_once(task)).map_or(true, |node| {
                atlas_state.saved_node_attachment(node);
                false
            })
        });
    }

    fn load(&mut self, node: AtlasNodeAttachment) {
        // Todo: build customizable loader abstraction
        self.loading_nodes
            .push(NodeAttachmentWithData::start_loading(
                node,
                self.path.clone(),
                self.texture_size,
                self.format,
                self.mip_level_count,
            ));
    }

    fn save(&mut self, node: AtlasNodeAttachment) {
        self.saving_nodes.push(
            NodeAttachmentWithData {
                node,
                data: self.data[node.atlas_index as usize].clone(),
                texture_size: self.texture_size,
            }
            .start_saving(self.path.clone()),
        );
    }

    fn sample(&self, lookup: NodeLookup) -> Vec4 {
        if lookup.atlas_index == INVALID_ATLAS_INDEX {
            return Vec4::splat(0.0); // Todo: Handle this better
        }

        let data = &self.data[lookup.atlas_index as usize];
        let coordinate = lookup.atlas_coordinate * self.scale + self.offset;

        data.sample(coordinate, self.texture_size)
    }
}

/// The internal representation of a present node in a [`NodeAtlas`].
#[derive(Clone)]
struct NodeState {
    /// The index of the node inside the atlas.
    atlas_index: u32,
    /// The count of [`Quadtrees`] that have requested this node.
    requests: u32,
    loaded_attachments: BitSet,
}

struct AttachmentGroup(BitSet);

pub(crate) struct NodeAtlasState {
    attachment_groups: Vec<AttachmentGroup>,
    node_states: HashMap<NodeCoordinate, NodeState>,
    unused_nodes: VecDeque<AtlasNode>,
    pub(crate) existing_nodes: HashSet<NodeCoordinate>,

    attachment_count: u32,

    to_load: VecDeque<AtlasNodeAttachment>,
    load_slots: u32,
    to_save: VecDeque<AtlasNodeAttachment>,
    pub(crate) save_slots: u32,
    pub(crate) max_save_slots: u32,

    pub(crate) download_slots: u32,
    pub(crate) max_download_slots: u32,

    pub(crate) max_atlas_write_slots: u32,
}

impl NodeAtlasState {
    fn new(
        atlas_size: u32,
        attachment_count: u32,
        attachment_groups: Vec<AttachmentGroup>,
        existing_nodes: HashSet<NodeCoordinate>,
    ) -> Self {
        let unused_nodes = (0..atlas_size)
            .map(|atlas_index| AtlasNode::new(NodeCoordinate::INVALID, atlas_index))
            .collect();

        Self {
            attachment_groups,
            node_states: default(),
            unused_nodes,
            existing_nodes,
            attachment_count,
            to_save: default(),
            to_load: default(),
            save_slots: 64,
            max_save_slots: 64,
            load_slots: 64,
            download_slots: 128,
            max_download_slots: 128,
            max_atlas_write_slots: 32,
        }
    }

    fn update(&mut self, attachments: &mut [AtlasAttachment]) {
        while self.save_slots > 0 {
            if let Some(node) = self.to_save.pop_front() {
                attachments[node.attachment_index as usize].save(node);
                self.save_slots -= 1;
            } else {
                break;
            }
        }

        while self.load_slots > 0 {
            if let Some(node) = self.to_load.pop_front() {
                attachments[node.attachment_index as usize].load(node);
                self.load_slots -= 1;
            } else {
                break;
            }
        }
    }

    fn loaded_node_attachment(&mut self, node: AtlasNodeAttachment) {
        self.load_slots += 1;

        let node_state = self.node_states.get_mut(&node.coordinate).unwrap();
        node_state
            .loaded_attachments
            .insert(node.attachment_index as usize);
    }

    fn saved_node_attachment(&mut self, _node: AtlasNodeAttachment) {
        self.save_slots += 1;
    }

    fn downloaded_node_attachment(&mut self, _node: AtlasNodeAttachment) {
        self.download_slots += 1;
    }

    fn get_node(&mut self, node_coordinate: NodeCoordinate) -> AtlasNode {
        if node_coordinate == NodeCoordinate::INVALID {
            return AtlasNode::new(NodeCoordinate::INVALID, INVALID_ATLAS_INDEX);
        }

        let atlas_index = if self.existing_nodes.contains(&node_coordinate) {
            self.node_states.get(&node_coordinate).unwrap().atlas_index
        } else {
            INVALID_ATLAS_INDEX
        };

        AtlasNode::new(node_coordinate, atlas_index)
    }

    fn allocate_node(&mut self) -> u32 {
        let unused_node = self.unused_nodes.pop_front().expect("Atlas out of indices");

        self.node_states.remove(&unused_node.coordinate);

        unused_node.atlas_index
    }

    fn get_or_allocate_node(&mut self, node_coordinate: NodeCoordinate) -> AtlasNode {
        if node_coordinate == NodeCoordinate::INVALID {
            return AtlasNode::new(NodeCoordinate::INVALID, INVALID_ATLAS_INDEX);
        }

        self.existing_nodes.insert(node_coordinate);

        let atlas_index = if let Some(node) = self.node_states.get(&node_coordinate) {
            node.atlas_index
        } else {
            let atlas_index = self.allocate_node();

            self.node_states.insert(
                node_coordinate,
                NodeState {
                    requests: 1,
                    atlas_index,
                    loaded_attachments: BitSet::new(),
                },
            );

            atlas_index
        };

        AtlasNode::new(node_coordinate, atlas_index)
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
                    atlas_index,
                    requests: 1,
                    loaded_attachments: BitSet::new(),
                },
            );

            for attachment_index in 0..self.attachment_count {
                self.to_load.push_back(AtlasNodeAttachment {
                    coordinate: node_coordinate,
                    atlas_index,
                    attachment_index,
                });
            }
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
            self.unused_nodes
                .push_back(AtlasNode::new(node_coordinate, node.atlas_index));
        }
    }

    fn get_best_node(&self, node_coordinate: NodeCoordinate, group_index: u32) -> QuadtreeEntry {
        let mut best_node_coordinate = node_coordinate;

        loop {
            if best_node_coordinate == NodeCoordinate::INVALID
                || best_node_coordinate.lod == INVALID_LOD
            {
                return QuadtreeEntry {
                    atlas_index: INVALID_ATLAS_INDEX,
                    atlas_lod: INVALID_LOD,
                };
            }

            if let Some(node_state) = self.node_states.get(&best_node_coordinate) {
                let group = &self.attachment_groups[group_index as usize];

                if group.0.is_subset(&node_state.loaded_attachments) {
                    return QuadtreeEntry {
                        atlas_index: node_state.atlas_index,
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
    pub(crate) attachments: Vec<AtlasAttachment>,
    // stores the attachment data
    pub(crate) state: NodeAtlasState,
    pub(crate) path: String,
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
        attachment_groups: &[Vec<u32>],
    ) -> Self {
        let attachments = attachments
            .iter()
            .map(|attachment| AtlasAttachment::new(attachment, atlas_size, path))
            .collect_vec();

        let attachment_groups = if attachment_groups.is_empty() {
            let mut set = BitSet::new();
            for index in 0..attachments.len() {
                set.insert(index);
            }
            vec![AttachmentGroup(set)]
        } else {
            attachment_groups
                .iter()
                .map(|group| {
                    let mut set = BitSet::new();
                    for &index in group {
                        set.insert(index as usize);
                    }
                    AttachmentGroup(set)
                })
                .collect_vec()
        };

        let existing_nodes = Self::load_node_config(path);

        let state = NodeAtlasState::new(
            atlas_size,
            attachments.len() as u32,
            attachment_groups,
            existing_nodes,
        );

        Self {
            attachments,
            state,
            path: path.to_string(),
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
            &config.attachment_groups,
        )
    }

    pub fn get_node(&mut self, node_coordinate: NodeCoordinate) -> AtlasNode {
        self.state.get_node(node_coordinate)
    }

    pub fn get_or_allocate_node(&mut self, node_coordinate: NodeCoordinate) -> AtlasNode {
        self.state.get_or_allocate_node(node_coordinate)
    }

    pub fn save(&mut self, node: AtlasNodeAttachment) {
        self.state.to_save.push_back(node);
    }

    pub(super) fn get_best_node(
        &self,
        node_coordinate: NodeCoordinate,
        group_index: u32,
    ) -> QuadtreeEntry {
        self.state.get_best_node(node_coordinate, group_index)
    }

    pub(super) fn sample_attachment(&self, node_lookup: NodeLookup, attachment_index: u32) -> Vec4 {
        self.attachments[attachment_index as usize].sample(node_lookup)
    }

    /// Updates the node atlas according to all corresponding quadtrees.
    pub(crate) fn update(
        mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
        view_query: Query<Entity, With<TerrainView>>,
        mut terrain_query: Query<(Entity, &mut NodeAtlas), With<Terrain>>,
    ) {
        for (terrain, mut node_atlas) in terrain_query.iter_mut() {
            let NodeAtlas {
                state, attachments, ..
            } = node_atlas.deref_mut();

            state.update(attachments);

            for attachment in attachments {
                attachment.update(state);
            }

            for view in view_query.iter() {
                if let Some(quadtree) = quadtrees.get_mut(&(terrain, view)) {
                    for node_coordinate in quadtree.released_nodes.drain(..) {
                        node_atlas.state.release_node(node_coordinate);
                    }

                    for node_coordinate in quadtree.requested_nodes.drain(..) {
                        node_atlas.state.request_node(node_coordinate);
                    }
                }
            }
        }
    }

    /// Saves the node configuration of the terrain, which stores the [`NodeCoordinate`]s of all the nodes
    /// of the terrain.
    pub(crate) fn save_node_config(&self) {
        let tc = TC {
            nodes: self
                .state
                .existing_nodes
                .iter()
                .map(|&node_coordinate| node_coordinate)
                .collect_vec(),
        };

        tc.save_file(format!("assets/{}/config.tc", &self.path))
            .unwrap();
    }

    /// Loads the node configuration of the terrain, which stores the [`NodeCoordinate`]s of all the nodes
    /// of the terrain.
    pub(crate) fn load_node_config(path: &str) -> HashSet<NodeCoordinate> {
        if let Ok(tc) = TC::load_file(format!("assets/{}/config.tc", path)) {
            tc.nodes.into_iter().collect()
        } else {
            println!("Node config not found.");
            HashSet::default()
        }
    }
}
