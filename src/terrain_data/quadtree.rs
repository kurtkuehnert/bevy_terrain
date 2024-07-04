use crate::{
    big_space::{GridTransformReadOnly, ReferenceFrames},
    math::{Coordinate, NodeCoordinate, TerrainModel},
    terrain::{Terrain, TerrainConfig},
    terrain_data::{node_atlas::NodeAtlas, sample_height, INVALID_ATLAS_INDEX, INVALID_LOD},
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
    util::inverse_mix,
};
use bevy::{
    math::{DVec2, DVec3},
    prelude::*,
};
use bytemuck::{Pod, Zeroable};
use itertools::iproduct;
use ndarray::{Array2, Array4};
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
    pub(super) atlas_uv: Vec2,
}

impl NodeLookup {
    pub(super) const INVALID: Self = Self {
        atlas_index: INVALID_ATLAS_INDEX,
        atlas_lod: INVALID_LOD,
        atlas_uv: Vec2::ZERO,
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
#[derive(Component)]
pub struct Quadtree {
    pub(super) origins: Array2<UVec2>,
    /// The current cpu quadtree data. This is synced each frame with the gpu quadtree data.
    pub(super) data: Array4<QuadtreeEntry>,
    /// Nodes that are no longer required by this quadtree.
    pub(super) released_nodes: Vec<NodeCoordinate>,
    /// Nodes that are requested to be loaded by this quadtree.
    pub(super) requested_nodes: Vec<NodeCoordinate>,
    /// The internal node states of the quadtree.
    nodes: Array4<QuadtreeNode>,
    /// The count of level of detail layers.
    lod_count: u32,
    side_count: u32,
    /// The count of nodes in x and y direction per layer.
    quadtree_size: u32,
    /// The distance (measured in node sizes) until which to request nodes to be loaded.
    load_distance: f32,
    blend_distance: f32,
    blend_range: f32,
    pub(crate) view_world_position: DVec3,
    pub(crate) model: TerrainModel,
    pub(crate) approximate_height: f32,
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
        model: TerrainModel,
        quadtree_size: u32,
        load_distance: f32,
        blend_distance: f32,
        blend_range: f32,
    ) -> Self {
        let side_count = model.side_count();

        Self {
            lod_count,
            side_count,
            quadtree_size,
            load_distance,
            blend_distance,
            blend_range,
            view_world_position: default(),
            approximate_height: (model.min_height + model.max_height) / 2.0,
            model,
            origins: Array2::default((side_count as usize, lod_count as usize)),
            data: Array4::default((
                side_count as usize,
                lod_count as usize,
                quadtree_size as usize,
                quadtree_size as usize,
            )),
            nodes: Array4::default((
                side_count as usize,
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
            config.model.clone(),
            view_config.quadtree_size,
            view_config.load_distance,
            view_config.blend_distance,
            view_config.blend_range,
        )
    }

    fn compute_quadtree_xy(coordinate: Coordinate, node_count: f64) -> DVec2 {
        // scale and clamp the coordinate to the quadtree bounds
        (coordinate.st * node_count).min(DVec2::splat(node_count - 0.000001))
    }

    fn compute_origin(&self, coordinate: Coordinate, lod: u32) -> UVec2 {
        let node_count = NodeCoordinate::node_count(lod) as f64;
        let quadtree_xy = Self::compute_quadtree_xy(coordinate, node_count);

        (quadtree_xy - 0.5 * self.quadtree_size as f64)
            .round()
            .clamp(
                DVec2::splat(0.0),
                DVec2::splat(node_count - self.quadtree_size as f64),
            )
            .as_uvec2()
    }

    fn compute_node_distance(&self, node: NodeCoordinate, view_coordinate: Coordinate) -> f64 {
        let node_count = NodeCoordinate::node_count(node.lod) as f64;
        let node_xy = IVec2::new(node.x as i32, node.y as i32);
        let view_node_xy = Self::compute_quadtree_xy(view_coordinate, node_count);
        let node_offset = view_node_xy.as_ivec2() - node_xy;
        let mut offset = view_node_xy % 1.0;

        if node_offset.x < 0 {
            offset.x = 0.0;
        } else if node_offset.x > 0 {
            offset.x = 1.0;
        }
        if node_offset.y < 0 {
            offset.y = 0.0;
        } else if node_offset.y > 0 {
            offset.y = 1.0;
        }

        let node_world_position =
            Coordinate::new(node.side, (node_xy.as_dvec2() + offset) / node_count)
                .world_position(&self.model, self.approximate_height);

        node_world_position.distance(self.view_world_position)
    }

    pub(super) fn compute_blend(&self, sample_world_position: DVec3) -> (u32, f32) {
        let view_distance = self.view_world_position.distance(sample_world_position);
        let target_lod = (self.blend_distance as f64 * self.model.scale() / view_distance)
            .log2()
            .min(self.lod_count as f64 - 0.00001) as f32;
        let lod = target_lod as u32;

        let ratio = if lod == 0 {
            0.0
        } else {
            inverse_mix(lod as f32 + self.blend_range, lod as f32, target_lod)
        };

        (lod, ratio)
    }

    pub(super) fn lookup_node(&self, world_position: DVec3, quadtree_lod: u32) -> NodeLookup {
        let coordinate = Coordinate::from_world_position(world_position, &self.model);

        let node_count = NodeCoordinate::node_count(quadtree_lod) as f64;
        let quadtree_xy = Self::compute_quadtree_xy(coordinate, node_count);

        let entry = self.data[[
            coordinate.side as usize,
            quadtree_lod as usize,
            quadtree_xy.x as usize % self.quadtree_size as usize,
            quadtree_xy.y as usize % self.quadtree_size as usize,
        ]];

        if entry.atlas_lod == INVALID_LOD {
            return NodeLookup::INVALID;
        }

        NodeLookup {
            atlas_index: entry.atlas_index,
            atlas_lod: entry.atlas_lod,
            atlas_uv: ((quadtree_xy / (1 << (quadtree_lod - entry.atlas_lod)) as f64) % 1.0)
                .as_vec2(),
        }
    }

    /// Traverses all quadtrees and updates the node states,
    /// while selecting newly requested and released nodes.
    pub(crate) fn compute_requests(
        mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
        view_query: Query<(Entity, GridTransformReadOnly), With<TerrainView>>,
        terrain_query: Query<(Entity, &TerrainConfig), With<Terrain>>,
        frames: ReferenceFrames,
    ) {
        for (terrain, config) in &terrain_query {
            let frame = frames.parent_frame(terrain).unwrap();

            for (view, view_transform) in &view_query {
                let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();

                quadtree.model = config.model.clone();
                quadtree.view_world_position = view_transform.position_double(frame);
                let view_coordinate =
                    Coordinate::from_world_position(quadtree.view_world_position, &quadtree.model);

                for side in 0..quadtree.side_count {
                    let view_coordinate = view_coordinate.project_to_side(side, &quadtree.model);

                    for lod in 0..quadtree.lod_count {
                        let origin = quadtree.compute_origin(view_coordinate, lod);
                        quadtree.origins[(side as usize, lod as usize)] = origin;

                        for (x, y) in
                            iproduct!(0..quadtree.quadtree_size, 0..quadtree.quadtree_size)
                        {
                            let node_coordinate = NodeCoordinate {
                                side,
                                lod,
                                x: origin.x + x,
                                y: origin.y + y,
                            };

                            let node_distance =
                                quadtree.compute_node_distance(node_coordinate, view_coordinate);
                            let load_distance = quadtree.load_distance as f64
                                * quadtree.model.scale()
                                / NodeCoordinate::node_count(node_coordinate.lod) as f64;

                            let state = if lod == 0 || node_distance < load_distance {
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
                    *entry = node_atlas.get_best_node(node.node_coordinate);
                }
            }
        }
    }

    pub(crate) fn approximate_height(
        mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
        view_query: Query<Entity, With<TerrainView>>,
        mut terrain_query: Query<(Entity, &NodeAtlas), With<Terrain>>,
    ) {
        for (terrain, node_atlas) in &mut terrain_query {
            for view in &view_query {
                let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();

                quadtree.approximate_height =
                    sample_height(quadtree, node_atlas, quadtree.view_world_position);
            }
        }
    }
}
