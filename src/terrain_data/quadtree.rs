use crate::{
    terrain::{Terrain, TerrainConfig},
    terrain_data::{
        coordinates::NodeCoordinate, coordinates::S2Coordinate, node_atlas::NodeAtlas,
        sample_attachment_local, INVALID_ATLAS_INDEX, INVALID_LOD, SIDE_COUNT,
    },
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
};
use bevy::{math::Vec4Swizzles, prelude::*};
use bytemuck::{Pod, Zeroable};
use itertools::iproduct;
use ndarray::Array4;
use std::iter;

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
struct QuadtreeNode {
    /// The current node coordinate at the quadtree position.
    node_coordinate: NodeCoordinate,
    /// Indicates, whether the node is currently demanded or released.
    state: RequestState,
}

impl Default for QuadtreeNode {
    fn default() -> Self {
        Self {
            node_coordinate: NodeCoordinate::INVALID,
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
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub(super) struct QuadtreeEntry {
    /// The atlas index of the best entry.
    pub(super) atlas_index: u32,
    /// The atlas lod of the best entry.
    pub(super) atlas_lod: u32,
}

impl Default for QuadtreeEntry {
    fn default() -> Self {
        Self {
            atlas_index: INVALID_ATLAS_INDEX,
            atlas_lod: INVALID_LOD,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(super) struct NodeLookup {
    pub(super) atlas_index: u32,
    pub(super) atlas_lod: u32,
    pub(super) atlas_coordinate: Vec2,
}

impl NodeLookup {
    pub(super) const INVALID: Self = Self {
        atlas_index: INVALID_ATLAS_INDEX,
        atlas_lod: INVALID_LOD,
        atlas_coordinate: Vec2::ZERO,
    };
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
/// This quadtree is a "cube" with a size of (`quadtree_size`x`quadtree_size`x`lod_count`), where each layer
/// corresponds to a lod. These layers are wrapping (modulo `quadtree_size`), that means that
/// the quadtree is always centered under the viewer and only considers `quadtree_size` / 2 nodes
/// in each direction.
///
/// Each frame the quadtree determines the state of each node via the
/// `compute_requests` methode.
/// After the [`NodeAtlas`] has adjusted to these requests, the quadtree retrieves the best
/// currently loaded nodes from the node atlas via the
/// `adjust` methode, which can later be used to access the terrain data.
#[derive(Default, Component)]
pub struct Quadtree {
    /// The current cpu quadtree data. This is synced each frame with the gpu quadtree data.
    pub(super) data: Array4<QuadtreeEntry>,
    /// Nodes that are no longer required by this quadtree.
    pub(super) released_nodes: Vec<NodeCoordinate>,
    /// Nodes that are requested to be loaded by this quadtree.
    pub(super) requested_nodes: Vec<NodeCoordinate>,
    /// The count of level of detail layers.
    lod_count: u32,
    /// The count of nodes in x and y direction per layer.
    quadtree_size: u32,
    /// The internal node states of the quadtree.
    nodes: Array4<QuadtreeNode>,
    /// The distance (measured in node sizes) until which to request nodes to be loaded.
    load_distance: f32,
    blend_distance: f32,
    blend_range: f32,
    inverse_model: Mat4,
    view_local_position: Vec3,
    min_height: f32,
    max_height: f32,
    approximate_height: f32,
}

impl Quadtree {
    /// Creates a new quadtree from parameters.
    ///
    /// * `lod_count` - The count of level of detail layers.
    /// * `quadtree_size` - The count of nodes in x and y direction per layer.
    /// * `center_size` - The size of the smallest nodes (with lod 0).
    /// * `load_distance` - The distance (measured in node sizes) until which to request nodes to be loaded.
    /// * `height` - The height of the terrain.
    pub fn new(
        lod_count: u32,
        quadtree_size: u32,
        load_distance: f32,
        blend_distance: f32,
        blend_range: f32,
        min_height: f32,
        max_height: f32,
    ) -> Self {
        Self {
            lod_count,
            quadtree_size,
            load_distance,
            blend_distance,
            blend_range,
            inverse_model: default(),
            min_height,
            max_height,
            approximate_height: (min_height + max_height) / 2.0,
            view_local_position: default(),
            data: Array4::default((
                SIDE_COUNT as usize,
                lod_count as usize,
                quadtree_size as usize,
                quadtree_size as usize,
            )),
            nodes: Array4::default((
                SIDE_COUNT as usize,
                lod_count as usize,
                quadtree_size as usize,
                quadtree_size as usize,
            )),
            released_nodes: default(),
            requested_nodes: default(),
        }
    }

    /// Creates a new quadtree from a terrain and a terrain view config.
    pub fn from_configs(config: &TerrainConfig, view_config: &TerrainViewConfig) -> Self {
        Self::new(
            config.lod_count,
            view_config.quadtree_size,
            view_config.load_distance,
            view_config.blend_distance,
            view_config.blend_range,
            config.min_height,
            config.max_height,
        )
    }

    #[inline]
    fn node_count(&self, lod: u32) -> u32 {
        1 << (self.lod_count - lod - 1)
    }

    #[inline]
    fn node_coordinate(&self, s2: S2Coordinate, lod: u32) -> Vec2 {
        let node_count = self.node_count(lod) as f32;
        let max_coordinate = Vec2::splat(node_count - 0.00001);

        (s2.st * node_count).clamp(Vec2::ZERO, max_coordinate)
    }

    fn origin(&self, quadtree_s2: S2Coordinate, lod: u32) -> UVec2 {
        let origin_node_coordinate = self.node_coordinate(quadtree_s2, lod);
        let max_offset = self.node_count(lod) as f32 - self.quadtree_size as f32;

        let quadtree_origin = (origin_node_coordinate - 0.5 * self.quadtree_size as f32)
            .round()
            .clamp(Vec2::splat(0.0), Vec2::splat(max_offset));

        quadtree_origin.as_uvec2()
    }

    pub(super) fn world_to_local_position(&self, world_position: Vec3) -> Vec3 {
        (self.inverse_model * world_position.extend(1.0)).xyz()
    }

    pub(super) fn compute_blend(&self, local_position: Vec3) -> (u32, f32) {
        let view_distance = local_position.distance(self.view_local_position);
        let blend_threshold_distance = self.blend_distance / (1 << (self.lod_count - 1)) as f32;
        let log_distance = (view_distance / blend_threshold_distance).log2().max(0.0);
        let ratio = 1.0 - (1.0 - log_distance % 1.0) / self.blend_range;

        (log_distance as u32, ratio)
    }

    pub(super) fn lookup_node(&self, local_position: Vec3, lod: u32) -> NodeLookup {
        let s2 = S2Coordinate::from_local_position(local_position);

        let quadtree_lod = lod.min(self.lod_count - 1);

        let mut node_coordinate = self.node_coordinate(s2, quadtree_lod);

        let entry = self.data[[
            s2.side as usize,
            quadtree_lod as usize,
            node_coordinate.x as usize % self.quadtree_size as usize,
            node_coordinate.y as usize % self.quadtree_size as usize,
        ]];

        if entry.atlas_lod == INVALID_LOD {
            return NodeLookup::INVALID;
        }

        node_coordinate /= (1 << (entry.atlas_lod - quadtree_lod)) as f32;

        NodeLookup {
            atlas_index: entry.atlas_index,
            atlas_lod: entry.atlas_lod,
            atlas_coordinate: node_coordinate % 1.0,
        }
    }

    /// Traverses all quadtrees and updates the node states,
    /// while selecting newly requested and released nodes.
    pub(crate) fn compute_requests(
        mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
        view_query: Query<(Entity, &GlobalTransform), With<TerrainView>>,
        terrain_query: Query<(Entity, &GlobalTransform), With<Terrain>>,
    ) {
        for (terrain, terrain_transform) in &terrain_query {
            for (view, view_transform) in &view_query {
                let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();

                let view_world_position = view_transform.translation();
                quadtree.inverse_model = terrain_transform.compute_matrix().inverse();

                quadtree.view_local_position =
                    quadtree.world_to_local_position(view_world_position);
                let view_s2 = S2Coordinate::from_local_position(quadtree.view_local_position);

                for side in 0..SIDE_COUNT {
                    #[cfg(feature = "spherical")]
                    let quadtree_s2 = view_s2.project_to_side(side);

                    #[cfg(not(feature = "spherical"))]
                    let quadtree_s2 = view_s2;

                    for lod in 0..quadtree.lod_count {
                        let node_count = quadtree.node_count(lod);
                        let quadtree_origin: UVec2 = quadtree.origin(quadtree_s2, lod);

                        for (x, y) in
                            iproduct!(0..quadtree.quadtree_size, 0..quadtree.quadtree_size)
                        {
                            let node_coordinate = NodeCoordinate {
                                side,
                                lod,
                                x: quadtree_origin.x + x,
                                y: quadtree_origin.y + y,
                            };

                            let node_s2 =
                                S2Coordinate::from_node_coordinate(node_coordinate, node_count);
                            let node_local_position = node_s2.to_local_position();

                            let distance =
                                node_local_position.distance(quadtree.view_local_position);
                            let node_distance = 0.5 * distance * node_count as f32;

                            let state = if node_distance < quadtree.load_distance {
                                RequestState::Requested
                            } else {
                                RequestState::Released
                            };

                            let node = &mut quadtree.nodes[[
                                side as usize,
                                lod as usize,
                                (node_coordinate.x % quadtree.quadtree_size) as usize,
                                (node_coordinate.y % quadtree.quadtree_size) as usize,
                            ]];

                            // check if quadtree slot refers to a new node
                            if node_coordinate != node.node_coordinate {
                                // release old node
                                if node.state == RequestState::Requested {
                                    node.state = RequestState::Released;
                                    quadtree.released_nodes.push(node.node_coordinate);
                                }

                                node.node_coordinate = node_coordinate;
                            }

                            // request or release node based on its distance to the view
                            match (node.state, state) {
                                (RequestState::Released, RequestState::Requested) => {
                                    node.state = RequestState::Requested;
                                    quadtree.requested_nodes.push(node.node_coordinate);
                                }
                                (RequestState::Requested, RequestState::Released) => {
                                    node.state = RequestState::Released;
                                    quadtree.released_nodes.push(node.node_coordinate);
                                }
                                (_, _) => {}
                            }
                        }
                    }
                }
            }
        }
    }

    /// Adjusts all quadtrees to their corresponding node atlas
    /// by updating the entries with the best available nodes.
    pub(crate) fn adjust_to_node_atlas(
        mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
        view_query: Query<Entity, With<TerrainView>>,
        mut terrain_query: Query<(Entity, &NodeAtlas), With<Terrain>>,
    ) {
        for (terrain, node_atlas) in &mut terrain_query {
            for view in &view_query {
                let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();

                for (node, entry) in iter::zip(&quadtree.nodes, &mut quadtree.data) {
                    *entry = node_atlas.get_best_node(node.node_coordinate, quadtree.lod_count);
                }
            }
        }
    }

    pub(crate) fn approximate_height(
        mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
        mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
        view_query: Query<Entity, With<TerrainView>>,
        mut terrain_query: Query<(Entity, &NodeAtlas), With<Terrain>>,
    ) {
        for (terrain, node_atlas) in &mut terrain_query {
            for view in &view_query {
                let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();
                let view_config = view_configs.get_mut(&(terrain, view)).unwrap();

                let local_position = Vec3::new(
                    quadtree.view_local_position.x,
                    quadtree.approximate_height,
                    quadtree.view_local_position.z,
                );

                let height = sample_attachment_local(quadtree, node_atlas, 0, local_position).x
                    * (quadtree.max_height - quadtree.min_height)
                    + quadtree.min_height;

                (quadtree.approximate_height, view_config.approximate_height) = (height, height);
            }
        }
    }
}
