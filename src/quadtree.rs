use crate::node_atlas::NodeAtlas;
use crate::{
    config::TerrainConfig,
    node_atlas::{AtlasIndex, INVALID_ATLAS_INDEX},
    Terrain, TerrainView, TerrainViewComponents,
};
use bevy::{math::Vec3Swizzles, prelude::*, utils::HashSet};
use itertools::iproduct;
use ndarray::Array3;
use std::collections::BTreeMap;
use std::{collections::VecDeque, mem};

// Todo: may be swap to u64 for giant terrains
// Todo: consider 3 bit face data, for cube sphere
/// A globally unique identifier of a node.
/// lod |  x |  y
///   4 | 14 | 14
pub(crate) type NodeId = u32;
pub(crate) const INVALID_NODE_ID: NodeId = NodeId::MAX;

/// An update to the [`GpuQuadtree`](crate::render::gpu_quadtree::GpuQuadtree).
/// The update packs the atlas index and atlas lod,
/// as well as the quadtree coordinate of the node.
/// Its representation is temporary unique for the corresponding view.
/// atlas_index | atlas_lod | lod | x | y
///          12 |         5 |   5 | 5 | 5
pub(crate) type NodeUpdate = u32;

/// The global coordinate of a node.
pub struct NodeCoordinate {
    pub lod: u32,
    pub x: u32,
    pub y: u32,
}

/// The cpu representation of a node.
pub struct Node {
    node_id: u32,            // current node id at the grid position
    requested: bool,         // whether the node should be requested
    atlas_index: AtlasIndex, // best active atlas index
    atlas_lod: u32,          // best active level of detail
}

impl Default for Node {
    fn default() -> Self {
        Self {
            node_id: INVALID_NODE_ID,
            requested: false,
            atlas_index: INVALID_ATLAS_INDEX,
            atlas_lod: u32::MAX,
        }
    }
}

impl Node {
    /// Calculates a unique identifier for the node at the specified coordinate.
    #[inline]
    pub(crate) fn id(lod: u32, x: u32, y: u32) -> NodeId {
        (lod & 0xF) << 28 | (x & 0x3FFF) << 14 | y & 0x3FFF
    }

    /// Calculates the coordinate of the node.
    #[inline]
    pub(crate) fn coordinate(id: NodeId) -> NodeCoordinate {
        NodeCoordinate {
            lod: (id >> 28) & 0xF,
            x: (id >> 14) & 0x3FFF,
            y: id & 0x3FFF,
        }
    }

    /// Calculates a node update that can be sent to the GPU.
    #[inline]
    fn update(atlas_index: AtlasIndex, atlas_lod: u32, lod: u32, x: u32, y: u32) -> NodeUpdate {
        (atlas_index as u32) << 20
            | (atlas_lod & 0x1F) << 15
            | (lod & 0x1F) << 10
            | (x & 0x1F) << 5
            | (y & 0x1F)
    }
}

// Todo: find a suitable name
/// A quadtree-like view of a terrain, that requests and releases nodes.
/// Additionally it tracks all [`NodeUpdate`]s, which are send to the GPU by the
/// [`GpuQuadtree`](crate::render::gpu_quadtree::GpuQuadtree).
#[derive(Component)]
pub struct Quadtree {
    //
    lod_count: u32,
    node_count: u32,
    chunk_size: u32,
    load_distance: f32,
    nodes: Array3<Node>,
    pub(crate) released_nodes: Vec<NodeId>,
    pub(crate) fallback_nodes: Vec<NodeId>,
    pub(crate) requested_nodes: Vec<NodeId>,
    pub(crate) waiting_nodes: HashSet<NodeId>,
    pub(crate) provided_nodes: BTreeMap<NodeId, AtlasIndex>,
    /// Nodes that should be loading or active.
    pub(crate) node_updates: Vec<NodeUpdate>,
}

impl Quadtree {
    /// Creates a new quadtree based on the supplied [`TerrainConfig`].
    pub fn new(config: &TerrainConfig) -> Self {
        Self {
            lod_count: config.lod_count,
            node_count: config.node_count,
            chunk_size: config.chunk_size,
            load_distance: config.load_distance,
            nodes: Array3::default((
                config.lod_count as usize,
                config.node_count as usize,
                config.node_count as usize,
            )),
            released_nodes: default(),
            fallback_nodes: default(),
            requested_nodes: default(),
            waiting_nodes: default(),
            provided_nodes: default(),
            node_updates: default(),
        }
    }

    #[inline]
    fn node_size(&self, lod: u32) -> u32 {
        self.chunk_size * (1 << lod)
    }

    fn update_node(&mut self, atlas_index: AtlasIndex, atlas_lod: u32, lod: u32, x: u32, y: u32) {
        let node = &mut self.nodes[[lod as usize, x as usize, y as usize]];
        node.atlas_index = atlas_index;
        node.atlas_lod = atlas_lod;

        let update = Node::update(atlas_index, atlas_lod, lod, x, y);
        self.node_updates.push(update);
    }

    /// Traverses the quadtree and selects all nodes to activate/deactivate.
    pub(crate) fn traverse(&mut self, viewer_position: Vec3) {
        // traverse the quadtree top down
        for lod in (0..self.lod_count).rev() {
            let node_size = self.node_size(lod);

            // bottom left position of grid in node coordinates
            let grid_coordinate: IVec2 = (viewer_position.xz() / node_size as f32 + 0.5
                - self.node_count as f32 / 2.0)
                .as_ivec2();

            for coordinate in iproduct!(0..self.node_count as i32, 0..self.node_count as i32)
                .filter_map(|(x, y)| {
                    let coordinate = grid_coordinate + IVec2::new(x, y);

                    if coordinate.x < 0 || coordinate.y < 0 {
                        None
                    } else {
                        Some(coordinate.as_uvec2())
                    }
                })
            {
                let node_id = Node::id(lod, coordinate.x, coordinate.y);
                let node = &mut self.nodes[[
                    lod as usize,
                    (coordinate.x % self.node_count) as usize,
                    (coordinate.y % self.node_count) as usize,
                ]];

                // quadtree slot refers to a new node
                if node_id != node.node_id {
                    // deactivate old node
                    if node.requested {
                        self.released_nodes.push(node.node_id);
                        self.waiting_nodes.remove(&node.node_id);
                        node.requested = false;
                    }

                    self.fallback_nodes.push(node_id);
                    node.node_id = node_id;
                }

                let node_position = (coordinate.as_vec2() + 0.5) * node_size as f32;
                let distance = viewer_position.xz().distance(node_position);
                let should_be_requested = distance < self.load_distance * node_size as f32;

                // Todo: always request highest lod
                // request or release node based on their distance to the viewer
                match (node.requested, should_be_requested) {
                    (false, true) => {
                        self.requested_nodes.push(node_id);
                        self.waiting_nodes.insert(node_id);
                        node.requested = true;
                    }
                    (true, false) => {
                        self.released_nodes.push(node_id);
                        self.waiting_nodes.remove(&node_id);
                        self.fallback_nodes.push(node_id);
                        node.requested = false;
                    }
                    (_, _) => {}
                }
            }
        }
    }

    fn compute_node_updates(&mut self) {
        let mut fallback_nodes = mem::take(&mut self.fallback_nodes);
        let mut provided_nodes = mem::take(&mut self.provided_nodes);

        // fall back to the closest present ancestor
        for node_id in fallback_nodes.drain(..) {
            let mut coordinate = Node::coordinate(node_id);

            let lod = coordinate.lod;
            let x = coordinate.x % self.node_count;
            let y = coordinate.y % self.node_count;

            loop {
                coordinate.lod += 1;
                coordinate.x >>= 1;
                coordinate.y >>= 1;

                if coordinate.lod == self.lod_count {
                    // dbg!("Could not fall back to any ancestor.");
                    break;
                }

                let ancestor_node = &self.nodes[[
                    coordinate.lod as usize,
                    (coordinate.x % self.node_count) as usize,
                    (coordinate.y % self.node_count) as usize,
                ]];

                let ancestor_id = Node::id(coordinate.lod, coordinate.x, coordinate.y);
                let atlas_index = ancestor_node.atlas_index;
                let atlas_lod = ancestor_node.atlas_lod;

                // the node is part of the quadtree
                if ancestor_id == ancestor_node.node_id {
                    self.update_node(atlas_index, atlas_lod, lod, x, y);
                    break;
                }
            }
        }

        // for each provided node update itself and its children
        for (&node_id, &atlas_index) in provided_nodes.iter() {
            let atlas_lod = Node::coordinate(node_id).lod;

            let mut queue = VecDeque::new();
            queue.push_back(node_id);

            while let Some(node_id) = queue.pop_front() {
                let coordinate = Node::coordinate(node_id);

                let lod = coordinate.lod;
                let x = coordinate.x % self.node_count;
                let y = coordinate.y % self.node_count;

                let node = &mut self.nodes[[lod as usize, x as usize, y as usize]];

                // the node is part of the quadtree and the atlas lod is an improvement
                if node_id == node.node_id && atlas_lod <= node.atlas_lod {
                    self.update_node(atlas_index, atlas_lod, lod, x, y);

                    // update children
                    if lod != 0 {
                        for (x, y) in iproduct!(0..2, 0..2) {
                            let child_id = Node::id(
                                coordinate.lod - 1,
                                (coordinate.x << 1) | x,
                                (coordinate.y << 1) | y,
                            );

                            queue.push_back(child_id);
                        }
                    }
                }
            }
        }
        provided_nodes.clear();

        self.fallback_nodes = fallback_nodes;
        self.provided_nodes = provided_nodes;
    }
}

/// Traverses all quadtrees and marks all nodes to activate/deactivate.
pub(crate) fn traverse_quadtree(
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<(Entity, &GlobalTransform), With<TerrainView>>,
    terrain_query: Query<(Entity, &GlobalTransform), With<Terrain>>,
) {
    // Todo: properly take the terrain transform into account
    for (terrain, _terrain_transform) in terrain_query.iter() {
        for (view, view_transform) in view_query.iter() {
            if let Some(quadtree) = quadtrees.get_mut(&(terrain, view)) {
                let view_position = view_transform.translation;

                quadtree.traverse(view_position);
            }
        }
    }
}

pub(crate) fn compute_node_updates(
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    mut terrain_query: Query<(Entity, &mut NodeAtlas), With<Terrain>>,
) {
    for (terrain, mut node_atlas) in terrain_query.iter_mut() {
        for view in view_query.iter() {
            if let Some(quadtree) = quadtrees.get_mut(&(terrain, view)) {
                node_atlas.update_quadtree(quadtree);
                quadtree.compute_node_updates();
            }
        }
    }
}
