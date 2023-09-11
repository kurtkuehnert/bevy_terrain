use crate::{
    terrain::{Terrain, TerrainConfig},
    terrain_data::{
        calc_node_id,
        node_atlas::{LoadingState, NodeAtlas},
        AtlasIndex, NodeCoordinate, NodeId, INVALID_ATLAS_INDEX, INVALID_LOD, INVALID_NODE_ID,
    },
    TerrainView, TerrainViewComponents, TerrainViewConfig,
};
use bevy::{math::Vec3Swizzles, prelude::*};
use bytemuck::{Pod, Zeroable};
use itertools::iproduct;
use ndarray::Array3;

/// The current state of a node of a [`Quadtree`].
///
/// This indicates, whether or not the node should be loaded into the [`NodeAtlas`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum RequestState {
    /// The node should be loaded.
    Requested,
    /// The node does not have to be loaded.
    Released,
}

/// The internal representation of a node in a [`Quadtree`].
struct TreeNode {
    /// The current node id at the quadtree position.
    node_id: NodeId,
    /// Indicates, whether the node is currently demanded or released.
    state: RequestState,
}

impl Default for TreeNode {
    fn default() -> Self {
        Self {
            node_id: INVALID_NODE_ID,
            state: RequestState::Released,
        }
    }
}

/// An entry of the [`Quadtree`], used to access the best currently loaded node
/// of the [`NodeAtlas`] on the CPU.
///
/// These entries are synced each frame with their equivalent representations in the
/// [`GpuQuadtree`](super::gpu_quadtree::GpuQuadtree) for access on the GPU.
#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub(crate) struct QuadtreeEntry {
    /// The atlas index of the best entry.
    atlas_index: AtlasIndex,
    /// The atlas lod of the best entry.
    atlas_lod: u16,
}

impl Default for QuadtreeEntry {
    fn default() -> Self {
        Self {
            atlas_index: INVALID_ATLAS_INDEX,
            atlas_lod: INVALID_LOD,
        }
    }
}

/// A quadtree-like view of a terrain, that requests and releases nodes from the [`NodeAtlas`]
/// depending on the distance to the viewer.
///
/// It can be used to access the best currently loaded node of the [`NodeAtlas`].
/// Additionally its sends this data to the GPU via the
/// [`GpuQuadtree`](super::gpu_quadtree::GpuQuadtree) so that it can be utilised
/// in shaders as well.
///
/// Each view (camera, shadow-casting light) that should consider the terrain has to
/// have an associated quadtree.
///
/// This quadtree is a "cube" with a size of (`node_count`x`node_count`x`lod_count`), where each layer
/// corresponds to a lod. These layers are wrapping (modulo `node_count`), that means that
/// the quadtree is always centered under the viewer and only considers `node_count` / 2 nodes
/// in each direction.
///
/// Each frame the quadtree determines the state of each node via the
/// `compute_requests` methode.
/// After the [`NodeAtlas`] has adjusted to these requests, the quadtree retrieves the best
/// currently loaded nodes from the node atlas via the
/// `adjust` methode, which can later be used to access the terrain data.
#[derive(Default, Component)]
pub struct Quadtree {
    /// The handle of the quadtree texture.
    pub(crate) handle: Handle<Image>,
    /// The current cpu quadtree data. This is synced each frame with the gpu quadtree data.
    pub(crate) data: Array3<QuadtreeEntry>,
    /// Nodes that are no longer required by this quadtree.
    pub(crate) released_nodes: Vec<NodeId>,
    /// Nodes that are requested to be loaded by this quadtree.
    pub(crate) requested_nodes: Vec<NodeId>,
    /// The count of level of detail layers.
    pub(crate) lod_count: u32,
    /// The count of nodes in x and y direction per layer.
    pub(crate) node_count: u32,
    /// The size of the smallest nodes (with lod 0).
    leaf_node_size: u32,
    /// The distance (measured in node sizes) until which to request nodes to be loaded.
    load_distance: f32,
    height: f32,
    height_under_viewer: f32,
    /// The internal node states of the quadtree.
    nodes: Array3<TreeNode>,
}

impl Quadtree {
    /// Creates a new quadtree from parameters.
    ///
    /// * `handle` - The handle of the quadtree texture.
    /// * `lod_count` - The count of level of detail layers.
    /// * `node_count` - The count of nodes in x and y direction per layer.
    /// * `leaf_node_size` - The size of the smallest nodes (with lod 0).
    /// * `load_distance` - The distance (measured in node sizes) until which to request nodes to be loaded.
    /// * `height` - The height of the terrain.
    pub fn new(
        handle: Handle<Image>,
        lod_count: u32,
        node_count: u32,
        leaf_node_size: u32,
        load_distance: f32,
        height: f32,
    ) -> Self {
        Self {
            handle,
            lod_count,
            node_count,
            leaf_node_size,
            load_distance,
            height,
            height_under_viewer: height / 2.0,
            data: Array3::default((lod_count as usize, node_count as usize, node_count as usize)),
            nodes: Array3::default((lod_count as usize, node_count as usize, node_count as usize)),
            released_nodes: default(),
            requested_nodes: default(),
        }
    }

    /// Creates a new quadtree from a terrain and a terrain view config.
    pub fn from_configs(config: &TerrainConfig, view_config: &TerrainViewConfig) -> Self {
        Self::new(
            view_config.quadtree_handle.clone(),
            config.lod_count,
            view_config.node_count,
            config.leaf_node_size,
            view_config.load_distance,
            config.height,
        )
    }

    /// Calculates the size of a node.
    #[inline]
    fn node_size(&self, lod: u32) -> u32 {
        self.leaf_node_size * (1 << lod)
    }

    /// Traverses the quadtree and updates the node states,
    /// while selecting newly requested and released nodes.
    pub(crate) fn compute_requests(&mut self, viewer_position: Vec3) {
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
                let node_id = calc_node_id(lod, coordinate.x, coordinate.y);
                let node = &mut self.nodes[[
                    lod as usize,
                    (coordinate.x % self.node_count) as usize,
                    (coordinate.y % self.node_count) as usize,
                ]];

                // quadtree slot refers to a new node
                if node_id != node.node_id {
                    // release old node
                    if node.state == RequestState::Requested {
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

                // request or release node based on their distance to the viewer
                match (node.state, demanded) {
                    (RequestState::Released, true) => {
                        self.requested_nodes.push(node.node_id);
                        node.state = RequestState::Requested;
                    }
                    (RequestState::Requested, false) => {
                        self.released_nodes.push(node.node_id);
                        node.state = RequestState::Released;
                    }
                    (_, _) => {}
                }
            }
        }
    }

    /// Adjusts the quadtree to the node atlas by updating the entries with the best available nodes.
    fn adjust(&mut self, node_atlas: &NodeAtlas) {
        for ((lod, x, y), node) in self.nodes.indexed_iter_mut() {
            let mut node_id = node.node_id;
            let mut coordinate = NodeCoordinate::from(node_id);

            let (atlas_index, atlas_lod) = loop {
                if coordinate.lod == self.lod_count || node_id == INVALID_NODE_ID {
                    // highest lod is not loaded
                    break (INVALID_ATLAS_INDEX, u16::MAX);
                }

                if let Some(atlas_node) = node_atlas.nodes.get(&node_id) {
                    if atlas_node.state == LoadingState::Loaded {
                        // found best loaded node
                        break (atlas_node.atlas_index, coordinate.lod as u16);
                    }
                }

                // node not loaded, try parent
                coordinate.lod += 1;
                coordinate.x >>= 1;
                coordinate.y >>= 1;
                node_id = calc_node_id(coordinate.lod, coordinate.x, coordinate.y);
            };

            self.data[[lod, y, x]] = QuadtreeEntry {
                atlas_index,
                atlas_lod,
            };
        }
    }
}

/// Traverses all quadtrees and updates the node states,
/// while selecting newly requested and released nodes.
pub(crate) fn compute_quadtree_request(
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<(Entity, &GlobalTransform), With<TerrainView>>,
    terrain_query: Query<(Entity, &GlobalTransform), With<Terrain>>,
) {
    // Todo: properly take the terrain transform into account
    for (terrain, _terrain_transform) in terrain_query.iter() {
        for (view, view_transform) in view_query.iter() {
            let view_position = view_transform.translation();
            let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();

            quadtree.compute_requests(view_position);
        }
    }
}




/// Adjusts all quadtrees to their corresponding node atlas
/// by updating the entries with the best available nodes.
pub(crate) fn adjust_quadtree(
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    mut terrain_query: Query<(Entity, &NodeAtlas), With<Terrain>>,
) {

    //consider removing this unwrap! 

    for (terrain, node_atlas) in terrain_query.iter_mut() {
        for view in view_query.iter() {
           // let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();

            match quadtrees.get_mut(&(terrain, view)) {

                Some(tree) => {
                    tree.adjust(node_atlas);
                },
                None => {
                    println!("WARN: Not adjusting based on quadtree");
                }

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
                    node_atlas,
                    &images,
                    view_transform.translation().xz(),
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
        (viewer_position / quadtree.leaf_node_size as f32).as_uvec2() % quadtree.node_count;

    let node = &quadtree.data[[0, coordinate.y as usize, coordinate.x as usize]];
    let atlas_coords = (viewer_position / quadtree.leaf_node_size as f32) % 1.0;

    if node.atlas_index == INVALID_ATLAS_INDEX {
        return 0.0;
    }

    if let Some(node) = node_atlas.data[node.atlas_index as usize]
        ._attachments
        .get(&0)
    {
        if let Some(image) = images.get(node) {
            let position = (image.size() * atlas_coords).as_uvec2();
            let index = 2 * (position.x + position.y * image.size().x as u32) as usize;
            let height = ((image.data[index + 1] as u16) << 8) + image.data[index] as u16;
            let height = height as f32 / u16::MAX as f32 * quadtree.height;

            return height;
        }
    }

    quadtree.height_under_viewer
}
