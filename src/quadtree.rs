use crate::node_atlas::LoadingState;
use crate::{
    node_atlas::{AtlasIndex, NodeAtlas, INVALID_ATLAS_INDEX},
    terrain::{Terrain, TerrainConfig},
    TerrainView, TerrainViewComponents, TerrainViewConfig,
};
use bevy::{math::Vec3Swizzles, prelude::*};
use itertools::iproduct;
use ndarray::Array3;

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RequestState {
    Demanded,
    Released,
}

/// The cpu representation of a node.
pub struct Node {
    node_id: u32, // current node id at the grid position
    state: RequestState,
    atlas_index: AtlasIndex, // best active atlas index
    atlas_lod: u32,          // best active level of detail
}

impl Default for Node {
    fn default() -> Self {
        Self {
            node_id: INVALID_NODE_ID,
            state: RequestState::Released,
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
    pub(crate) lod_count: u32,
    pub(crate) node_count: u32,
    chunk_size: u32,
    load_distance: f32,
    height: f32,
    height_under_viewer: f32,
    nodes: Array3<Node>,
    pub(crate) released_nodes: Vec<NodeId>,
    pub(crate) demanded_nodes: Vec<NodeId>,
    /// Nodes that should be loading or active.
    pub(crate) node_updates: Vec<NodeUpdate>,
}

impl Quadtree {
    /// Creates a new quadtree based on the supplied [`TerrainConfig`].
    pub fn new(config: &TerrainConfig, view_config: &TerrainViewConfig) -> Self {
        Self {
            lod_count: config.lod_count,
            node_count: view_config.node_count,
            chunk_size: config.chunk_size,
            load_distance: view_config.load_distance,
            height: config.height,
            height_under_viewer: config.height / 2.0,
            nodes: Array3::default((
                config.lod_count as usize,
                view_config.node_count as usize,
                view_config.node_count as usize,
            )),
            released_nodes: default(),
            demanded_nodes: default(),
            node_updates: default(),
        }
    }

    #[inline]
    fn node_size(&self, lod: u32) -> u32 {
        self.chunk_size * (1 << lod)
    }

    /// Traverses the quadtree and selects all nodes to activate/deactivate.
    pub(crate) fn request(&mut self, viewer_position: Vec3) {
        for lod in 0..self.lod_count {
            let node_size = self.node_size(lod);

            // bottom left position of grid in node coordinates
            let grid_coordinate: IVec2 = (viewer_position.xz() / node_size as f32 + 0.5
                - (self.node_count >> 1) as f32)
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
                    // release old node
                    if node.state == RequestState::Demanded {
                        self.released_nodes.push(node.node_id);
                        node.state = RequestState::Released;
                    }

                    node.node_id = node_id;
                }

                let node_position = (coordinate.as_vec2() + 0.5) * node_size as f32;
                let world_position =
                    Vec3::new(node_position.x, self.height_under_viewer, node_position.y);
                let distance = viewer_position.xyz().distance(world_position);
                let mut demanded = distance < self.load_distance * node_size as f32;
                demanded |= lod == self.lod_count - 1; // always request highest lod

                // demand or release node based on their distance to the viewer
                match (node.state, demanded) {
                    (RequestState::Released, true) => {
                        self.demanded_nodes.push(node.node_id);
                        node.state = RequestState::Demanded;
                    }
                    (RequestState::Demanded, false) => {
                        self.released_nodes.push(node.node_id);
                        node.state = RequestState::Released;
                    }
                    (_, _) => {}
                }
            }
        }
    }

    fn adjust(&mut self, node_atlas: &NodeAtlas) {
        self.node_updates.clear();
        for ((lod, x, y), node) in self.nodes.indexed_iter_mut() {
            let mut node_id = node.node_id;
            let mut coordinate = Node::coordinate(node_id);

            let (atlas_index, atlas_lod) = loop {
                if coordinate.lod == self.lod_count || node_id == INVALID_NODE_ID {
                    // highest lod is not loaded
                    break (INVALID_ATLAS_INDEX, u32::MAX);
                }

                if let Some(atlas_node) = node_atlas.nodes.get(&node_id) {
                    if atlas_node.state == LoadingState::Loaded {
                        // found best loaded node
                        break (atlas_node.atlas_index, coordinate.lod);
                    }
                }

                // node not loaded, try parent
                coordinate.lod += 1;
                coordinate.x >>= 1;
                coordinate.y >>= 1;
                node_id = Node::id(coordinate.lod, coordinate.x, coordinate.y);
            };

            node.atlas_index = atlas_index;
            node.atlas_lod = atlas_lod;
            self.node_updates.push(Node::update(
                atlas_index,
                atlas_lod,
                lod as u32,
                x as u32,
                y as u32,
            ));
        }
    }
}

/// Traverses all quadtrees and marks all nodes to activate/deactivate.
pub(crate) fn request_quadtree(
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<(Entity, &GlobalTransform), With<TerrainView>>,
    terrain_query: Query<(Entity, &GlobalTransform), With<Terrain>>,
) {
    // Todo: properly take the terrain transform into account
    for (terrain, _terrain_transform) in terrain_query.iter() {
        for (view, view_transform) in view_query.iter() {
            if let Some(quadtree) = quadtrees.get_mut(&(terrain, view)) {
                let view_position = view_transform.translation;

                quadtree.request(view_position);
            }
        }
    }
}

pub(crate) fn adjust_quadtree(
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    mut terrain_query: Query<(Entity, &NodeAtlas), With<Terrain>>,
) {
    for (terrain, mut node_atlas) in terrain_query.iter_mut() {
        for view in view_query.iter() {
            if let Some(quadtree) = quadtrees.get_mut(&(terrain, view)) {
                quadtree.adjust(&mut node_atlas);
            }
        }
    }
}

pub(crate) fn update_height_under_viewer(
    images: Res<Assets<Image>>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    mut terrain_view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
    view_query: Query<(Entity, &GlobalTransform), With<TerrainView>>,
    mut terrain_query: Query<(Entity, &NodeAtlas), With<Terrain>>,
) {
    for (terrain, node_atlas) in terrain_query.iter_mut() {
        for (view, view_transform) in view_query.iter() {
            if let Some(quadtree) = quadtrees.get_mut(&(terrain, view)) {
                quadtree.height_under_viewer = height_under_viewer(
                    quadtree,
                    &node_atlas,
                    &images,
                    view_transform.translation.xz(),
                );

                terrain_view_configs
                    .get_mut(&(terrain, view))
                    .unwrap()
                    .height_under_viewer = quadtree.height_under_viewer;
            }
        }
    }
}

fn height_under_viewer(
    quadtree: &Quadtree,
    node_atlas: &NodeAtlas,
    images: &Assets<Image>,
    viewer_position: Vec2,
) -> f32 {
    let coordinate =
        (viewer_position / quadtree.chunk_size as f32).as_uvec2() % quadtree.node_count;

    let node = &quadtree.nodes[[0, coordinate.x as usize, coordinate.y as usize]];
    let atlas_coords = (viewer_position / quadtree.chunk_size as f32) % 1.0;

    if node.atlas_index == INVALID_ATLAS_INDEX {
        return 0.0;
    }

    let node = node_atlas.data[node.atlas_index as usize]
        ._attachments
        .get(&0)
        .unwrap();

    let image = images.get(node).unwrap();

    let position = (image.size() * atlas_coords).as_uvec2();
    let index = 2 * (position.x + position.y * image.size().x as u32) as usize;
    let height = ((image.data[index + 1] as u16) << 8) + image.data[index] as u16;
    let height = height as f32 / u16::MAX as f32 * quadtree.height;

    return height;
}
