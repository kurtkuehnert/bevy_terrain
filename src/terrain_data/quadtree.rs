use crate::{
    terrain::{Terrain, TerrainConfig},
    terrain_data::{
        node_atlas::{LoadingState, NodeAtlas},
        AtlasIndex, NodeCoordinate, INVALID_ATLAS_INDEX, INVALID_LOD, SIDE_COUNT,
    },
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
};
use bevy::{math::Vec3Swizzles, prelude::*};
use bytemuck::{Pod, Zeroable};
use itertools::iproduct;
use ndarray::{Array3, Array4};

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

#[derive(Clone, Copy)]
struct S2Coordinate {
    side: u32,
    st: Vec2,
}

impl S2Coordinate {
    #[cfg(feature = "spherical")]
    fn from_world_position(world_position: Vec3, _quadtree: &Quadtree) -> Self {
        let local_position = world_position.xyz();

        let direction = local_position.normalize();
        let abs_direction = direction.abs();

        let (side, uv) = if abs_direction.x > abs_direction.y && abs_direction.x > abs_direction.z {
            if direction.x < 0.0 {
                (
                    0,
                    Vec2::new(-direction.z / direction.x, direction.y / direction.x),
                )
            } else {
                (
                    3,
                    Vec2::new(-direction.y / direction.x, direction.z / direction.x),
                )
            }
        } else if abs_direction.z > abs_direction.y {
            if direction.z > 0.0 {
                (
                    1,
                    Vec2::new(direction.x / direction.z, -direction.y / direction.z),
                )
            } else {
                (
                    4,
                    Vec2::new(direction.y / direction.z, -direction.x / direction.z),
                )
            }
        } else {
            if direction.y > 0.0 {
                (
                    2,
                    Vec2::new(direction.x / direction.y, direction.z / direction.y),
                )
            } else {
                (
                    5,
                    Vec2::new(-direction.z / direction.y, -direction.x / direction.y),
                )
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
    fn from_world_position(world_position: Vec3, quadtree: &Quadtree) -> Self {
        let local_position = world_position.xyz();

        let st = local_position.xz() / quadtree.terrain_size + 0.5;

        Self { side: 0, st }
    }

    fn from_node_coordinate(node_coordinate: NodeCoordinate, nodes_per_side: f32) -> Self {
        let st = (Vec2::new(
            node_coordinate.x as f32 + 0.5,
            node_coordinate.y as f32 + 0.5,
        )) / nodes_per_side;

        Self {
            side: node_coordinate.side,
            st,
        }
    }

    #[cfg(feature = "spherical")]
    fn to_world_position(self, quadtree: &Quadtree) -> Vec3 {
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

        Self::uv_to_world_position(uv, self.side, quadtree)
    }

    #[cfg(not(feature = "spherical"))]
    fn to_world_position(self, quadtree: &Quadtree) -> Vec3 {
        let local_position = (self.st - 0.5) * quadtree.terrain_size;

        let world_position = Vec3::new(local_position.x, 0.0, local_position.y);

        world_position
    }

    fn uv_to_world_position(uv: Vec2, side: u32, quadtree: &Quadtree) -> Vec3 {
        let local_position = match side {
            0 => Vec3::new(-1.0, -uv.y, uv.x),
            1 => Vec3::new(uv.x, -uv.y, 1.0),
            2 => Vec3::new(uv.x, 1.0, uv.y),
            3 => Vec3::new(1.0, -uv.x, uv.y),
            4 => Vec3::new(uv.y, -uv.x, -1.0),
            5 => Vec3::new(uv.y, -1.0, uv.x),
            _ => unreachable!(),
        }
        .normalize();

        let world_position = local_position * quadtree.radius;

        world_position
    }

    fn project_to_side(self, side: u32) -> Self {
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
struct TreeNode {
    /// The current node coordinate at the quadtree position.
    node_coordinate: NodeCoordinate,
    /// Indicates, whether the node is currently demanded or released.
    state: RequestState,
}

impl Default for TreeNode {
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
    pub(crate) released_nodes: Vec<NodeCoordinate>,
    /// Nodes that are requested to be loaded by this quadtree.
    pub(crate) requested_nodes: Vec<NodeCoordinate>,
    /// The count of level of detail layers.
    pub(crate) lod_count: u32,
    /// The count of nodes in x and y direction per layer.
    pub(crate) node_count: u32,
    nodes_per_side: f32,
    terrain_size: f32,
    radius: f32,
    /// The distance (measured in node sizes) until which to request nodes to be loaded.
    load_distance: f32,
    _height: f32,
    _height_under_viewer: f32,
    /// The internal node states of the quadtree.
    nodes: Array4<TreeNode>,
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
        nodes_per_side: f32,
        load_distance: f32,
        height: f32,
        terrain_size: f32,
        radius: f32,
    ) -> Self {
        Self {
            handle,
            lod_count,
            node_count,
            nodes_per_side,
            terrain_size,
            radius,
            load_distance,
            _height: height,
            _height_under_viewer: height / 2.0,
            data: Array3::default((
                SIDE_COUNT as usize * lod_count as usize,
                node_count as usize,
                node_count as usize,
            )),
            nodes: Array4::default((
                SIDE_COUNT as usize,
                lod_count as usize,
                node_count as usize,
                node_count as usize,
            )),
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
            config.nodes_per_side,
            view_config.load_distance,
            config.height,
            config.terrain_size,
            config.radius,
        )
    }

    #[inline]
    fn nodes_per_side(&self, lod: u32) -> f32 {
        self.nodes_per_side / (1 << lod) as f32
    }

    fn origin(&self, quadtree_s2: S2Coordinate, lod: u32) -> UVec2 {
        let nodes_per_side = self.nodes_per_side(lod);
        let origin_node_coordinate = quadtree_s2.st * nodes_per_side;
        let quadtree_size = self.node_count as f32;
        let max_size = nodes_per_side.ceil() - quadtree_size;

        let quadtree_origin = (origin_node_coordinate - 0.5 * quadtree_size)
            .round()
            .clamp(Vec2::splat(0.0), Vec2::splat(max_size));

        quadtree_origin.as_uvec2()
    }

    pub(crate) fn compute_requests(&mut self, view_position: Vec3) {
        let view_s2 = S2Coordinate::from_world_position(view_position, self);

        for side in 0..SIDE_COUNT {
            let quadtree_s2 = view_s2.project_to_side(side);

            for lod in 0..self.lod_count {
                let quadtree_origin: UVec2 = self.origin(quadtree_s2, lod);

                for (x, y) in iproduct!(0..self.node_count, 0..self.node_count) {
                    let new_node_coordinate = NodeCoordinate {
                        side,
                        lod,
                        x: quadtree_origin.x + x,
                        y: quadtree_origin.y + y,
                    };

                    // Todo: figure out whether to request or release the node based on viewer distance
                    let s2 = S2Coordinate::from_node_coordinate(
                        new_node_coordinate,
                        self.nodes_per_side(lod),
                    );
                    let world_position = s2.to_world_position(self);
                    let distance = world_position.distance(view_position);

                    let new_state = if distance < self.load_distance * 2.0_f32.powi(lod as i32) {
                        RequestState::Requested
                    } else {
                        RequestState::Released
                    };

                    // let new_state = RequestState::Requested;

                    let node = &mut self.nodes[[
                        side as usize,
                        lod as usize,
                        (new_node_coordinate.x % self.node_count) as usize,
                        (new_node_coordinate.y % self.node_count) as usize,
                    ]];

                    // check if quadtree slot refers to a new node
                    if new_node_coordinate != node.node_coordinate {
                        // release old node
                        if node.state == RequestState::Requested {
                            node.state = RequestState::Released;
                            self.released_nodes.push(node.node_coordinate);
                        }

                        node.node_coordinate = new_node_coordinate;
                    }

                    // request or release node based on its distance to the view
                    match (node.state, new_state) {
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

    /// Adjusts the quadtree to the node atlas by updating the entries with the best available nodes.
    fn adjust(&mut self, node_atlas: &NodeAtlas) {
        for ((side, lod, x, y), node) in self.nodes.indexed_iter_mut() {
            let mut best_node_coordinate = node.node_coordinate;

            let (atlas_index, atlas_lod) = loop {
                if best_node_coordinate == NodeCoordinate::INVALID
                    || best_node_coordinate.lod == self.lod_count
                {
                    // highest lod is not loaded
                    break (INVALID_ATLAS_INDEX, u16::MAX);
                }

                if let Some(atlas_node) = node_atlas.nodes.get(&best_node_coordinate) {
                    if atlas_node.state == LoadingState::Loaded {
                        // found best loaded node
                        break (atlas_node.atlas_index, best_node_coordinate.lod as u16);
                    }
                }

                // node not loaded, try parent
                best_node_coordinate.lod += 1;
                best_node_coordinate.x >>= 1;
                best_node_coordinate.y >>= 1;
            };

            self.data[[side * self.lod_count as usize + lod, y, x]] = QuadtreeEntry {
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
    for (terrain, node_atlas) in terrain_query.iter_mut() {
        for view in view_query.iter() {
            let quadtree = quadtrees.get_mut(&(terrain, view)).unwrap();

            quadtree.adjust(node_atlas);
        }
    }
}

/*
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
*/
