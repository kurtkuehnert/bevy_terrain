use crate::{
    terrain::{Terrain, TerrainConfig},
    terrain_data::{
        node_atlas::NodeAtlas, NodeCoordinate, INVALID_ATLAS_INDEX, INVALID_LOD, SIDE_COUNT,
    },
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
};
use bevy::{math::Vec4Swizzles, prelude::*};
use bytemuck::{Pod, Zeroable};
use itertools::iproduct;
use ndarray::Array4;
use std::iter;

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct S2Coordinate {
    side: u32,
    st: Vec2,
}

impl S2Coordinate {
    fn from_node_coordinate(node_coordinate: NodeCoordinate, node_count: f32) -> Self {
        let st = (Vec2::new(
            node_coordinate.x as f32 + 0.5,
            node_coordinate.y as f32 + 0.5,
        )) / node_count;

        Self {
            side: node_coordinate.side,
            st,
        }
    }

    fn from_local_position(local_position: Vec3) -> Self {
        #[cfg(feature = "spherical")]
        {
            let normal = local_position.normalize();
            let abs_normal = normal.abs();

            let (side, uv) = if abs_normal.x > abs_normal.y && abs_normal.x > abs_normal.z {
                if normal.x < 0.0 {
                    (0, Vec2::new(-normal.z / normal.x, normal.y / normal.x))
                } else {
                    (3, Vec2::new(-normal.y / normal.x, normal.z / normal.x))
                }
            } else if abs_normal.z > abs_normal.y {
                if normal.z > 0.0 {
                    (1, Vec2::new(normal.x / normal.z, -normal.y / normal.z))
                } else {
                    (4, Vec2::new(normal.y / normal.z, -normal.x / normal.z))
                }
            } else {
                if normal.y > 0.0 {
                    (2, Vec2::new(normal.x / normal.y, normal.z / normal.y))
                } else {
                    (5, Vec2::new(-normal.z / normal.y, -normal.x / normal.y))
                }
            };

            let st = uv
                .to_array()
                .map(|f| {
                    if f > 0.0 {
                        0.5 * (1.0 + 3.0 * f).sqrt()
                    } else {
                        1.0 - 0.5 * (1.0 - 3.0 * f).sqrt()
                    }
                })
                .into();

            Self { side, st }
        }

        #[cfg(not(feature = "spherical"))]
        return Self {
            side: 0,
            st: Vec2::new(0.5 * local_position.x + 0.5, 0.5 * local_position.z + 0.5),
        };
    }

    fn to_local_position(self) -> Vec3 {
        #[cfg(feature = "spherical")]
        {
            let uv: Vec2 = self
                .st
                .to_array()
                .map(|f| {
                    if f > 0.5 {
                        (4.0 * f.powi(2) - 1.0) / 3.0
                    } else {
                        (1.0 - 4.0 * (1.0 - f).powi(2)) / 3.0
                    }
                })
                .into();

            match self.side {
                0 => Vec3::new(-1.0, -uv.y, uv.x),
                1 => Vec3::new(uv.x, -uv.y, 1.0),
                2 => Vec3::new(uv.x, 1.0, uv.y),
                3 => Vec3::new(1.0, -uv.x, uv.y),
                4 => Vec3::new(uv.y, -uv.x, -1.0),
                5 => Vec3::new(uv.y, -1.0, uv.x),
                _ => unreachable!(),
            }
            .normalize()
        }

        #[cfg(not(feature = "spherical"))]
        return Vec3::new(2.0 * self.st.x - 1.0, 0.0, 2.0 * self.st.y - 1.0);
    }

    #[cfg(feature = "spherical")]
    fn project_to_side(self, side: u32) -> Self {
        #[derive(Clone, Copy)]
        enum SideInfo {
            Fixed0,
            Fixed1,
            PositiveS,
            PositiveT,
        }

        impl SideInfo {
            const EVEN_LIST: [[SideInfo; 2]; 6] = [
                [SideInfo::PositiveS, SideInfo::PositiveT],
                [SideInfo::Fixed0, SideInfo::PositiveT],
                [SideInfo::Fixed0, SideInfo::PositiveS],
                [SideInfo::PositiveT, SideInfo::PositiveS],
                [SideInfo::PositiveT, SideInfo::Fixed0],
                [SideInfo::PositiveS, SideInfo::Fixed0],
            ];
            const ODD_LIST: [[SideInfo; 2]; 6] = [
                [SideInfo::PositiveS, SideInfo::PositiveT],
                [SideInfo::PositiveS, SideInfo::Fixed1],
                [SideInfo::PositiveT, SideInfo::Fixed1],
                [SideInfo::PositiveT, SideInfo::PositiveS],
                [SideInfo::Fixed1, SideInfo::PositiveS],
                [SideInfo::Fixed1, SideInfo::PositiveT],
            ];
        }

        let index = ((6 + side - self.side) % 6) as usize;

        let info = if self.side % 2 == 0 {
            SideInfo::EVEN_LIST[index]
        } else {
            SideInfo::ODD_LIST[index]
        };

        let st = info
            .map(|info| match info {
                SideInfo::Fixed0 => 0.0,
                SideInfo::Fixed1 => 1.0,
                SideInfo::PositiveS => self.st.x,
                SideInfo::PositiveT => self.st.y,
            })
            .into();

        Self { side, st }
    }
}

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
#[derive(Clone, Copy, Zeroable, Pod)]
pub(crate) struct QuadtreeEntry {
    /// The atlas index of the best entry.
    pub(crate) atlas_index: u32,
    /// The atlas lod of the best entry.
    pub(crate) atlas_lod: u32,
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
    pub(crate) data: Array4<QuadtreeEntry>,
    /// Nodes that are no longer required by this quadtree.
    pub(crate) released_nodes: Vec<NodeCoordinate>,
    /// Nodes that are requested to be loaded by this quadtree.
    pub(crate) requested_nodes: Vec<NodeCoordinate>,
    /// The count of level of detail layers.
    pub(crate) lod_count: u32,
    /// The count of nodes in x and y direction per layer.
    pub(crate) quadtree_size: u32,
    /// The internal node states of the quadtree.
    nodes: Array4<QuadtreeNode>,
    /// The distance (measured in node sizes) until which to request nodes to be loaded.
    load_distance: f32,
    _height: f32,
    _height_under_viewer: f32,
}

impl Quadtree {
    /// Creates a new quadtree from parameters.
    ///
    /// * `lod_count` - The count of level of detail layers.
    /// * `quadtree_size` - The count of nodes in x and y direction per layer.
    /// * `center_size` - The size of the smallest nodes (with lod 0).
    /// * `load_distance` - The distance (measured in node sizes) until which to request nodes to be loaded.
    /// * `height` - The height of the terrain.
    pub fn new(lod_count: u32, quadtree_size: u32, load_distance: f32, height: f32) -> Self {
        Self {
            lod_count,
            quadtree_size,
            load_distance,
            _height: height,
            _height_under_viewer: height / 2.0,
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
            config.max_height,
        )
    }

    #[inline]
    fn node_count(&self, lod: u32) -> f32 {
        (1 << (self.lod_count - lod - 1)) as f32
    }

    fn origin(&self, quadtree_s2: S2Coordinate, lod: u32) -> UVec2 {
        let origin_node_coordinate = quadtree_s2.st * self.node_count(lod);
        let max_offset = self.node_count(lod).ceil() - self.quadtree_size as f32;

        let quadtree_origin = (origin_node_coordinate - 0.5 * self.quadtree_size as f32)
            .round()
            .clamp(Vec2::splat(0.0), Vec2::splat(max_offset));

        quadtree_origin.as_uvec2()
    }

    fn world_to_local_position(&self, transform: &GlobalTransform, world_position: Vec3) -> Vec3 {
        let transform = transform.compute_matrix();
        let inverse_model = transform.inverse();

        (inverse_model * world_position.extend(1.0)).xyz()
    }

    pub(crate) fn compute_requests(
        &mut self,
        terrain_transform: &GlobalTransform,
        view_world_position: Vec3,
    ) {
        let view_local_position =
            self.world_to_local_position(terrain_transform, view_world_position);
        let view_s2 = S2Coordinate::from_local_position(view_local_position);

        for side in 0..SIDE_COUNT {
            #[cfg(feature = "spherical")]
            let quadtree_s2 = view_s2.project_to_side(side);

            #[cfg(not(feature = "spherical"))]
            let quadtree_s2 = view_s2;

            for lod in 0..self.lod_count {
                let node_count = self.node_count(lod);
                let quadtree_origin: UVec2 = self.origin(quadtree_s2, lod);

                for (x, y) in iproduct!(0..self.quadtree_size, 0..self.quadtree_size) {
                    let node_coordinate = NodeCoordinate {
                        side,
                        lod,
                        x: quadtree_origin.x + x,
                        y: quadtree_origin.y + y,
                    };

                    let node_s2 = S2Coordinate::from_node_coordinate(node_coordinate, node_count);
                    let node_local_position = node_s2.to_local_position();

                    let distance = node_local_position.distance(view_local_position);
                    let node_distance = 0.5 * distance * node_count;

                    let state = if node_distance < self.load_distance {
                        RequestState::Requested
                    } else {
                        RequestState::Released
                    };

                    let node = &mut self.nodes[[
                        side as usize,
                        lod as usize,
                        (node_coordinate.x % self.quadtree_size) as usize,
                        (node_coordinate.y % self.quadtree_size) as usize,
                    ]];

                    // check if quadtree slot refers to a new node
                    if node_coordinate != node.node_coordinate {
                        // release old node
                        if node.state == RequestState::Requested {
                            node.state = RequestState::Released;
                            self.released_nodes.push(node.node_coordinate);
                        }

                        node.node_coordinate = node_coordinate;
                    }

                    // request or release node based on its distance to the view
                    match (node.state, state) {
                        (RequestState::Released, RequestState::Requested) => {
                            node.state = RequestState::Requested;
                            self.requested_nodes.push(node.node_coordinate);
                        }
                        (RequestState::Requested, RequestState::Released) => {
                            node.state = RequestState::Released;
                            self.released_nodes.push(node.node_coordinate);
                        }
                        (_, _) => {}
                    }
                }
            }
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
    for ((terrain, terrain_transform), (view, view_transform)) in
        iter::zip(&terrain_query, &view_query)
    {
        let view_position = view_transform.translation();
        let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();

        quadtree.compute_requests(terrain_transform, view_position);
    }
}

/// Adjusts all quadtrees to their corresponding node atlas
/// by updating the entries with the best available nodes.
pub(crate) fn adjust_quadtree(
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    mut terrain_query: Query<(Entity, &NodeAtlas), With<Terrain>>,
) {
    for ((terrain, node_atlas), view) in iter::zip(&mut terrain_query, &view_query) {
        let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();

        for (node, entry) in iter::zip(&quadtree.nodes, &mut quadtree.data) {
            *entry = node_atlas.get_best_node(node.node_coordinate, quadtree.lod_count);
        }
    }
}
