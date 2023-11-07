use crate::{
    terrain::{Terrain, TerrainConfig},
    terrain_data::{
        node_atlas::{LoadingState, NodeAtlas},
        AtlasIndex, NodeCoordinate, INVALID_ATLAS_INDEX, INVALID_LOD, SIDE_COUNT,
    },
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
};
use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};
use itertools::iproduct;
use ndarray::{Array3, Array4};

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
    pub(crate) quadtree_size: u32,
    leaf_node_count: f32,
    leaf_node_size: f32,
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
    /// * `quadtree_size` - The count of nodes in x and y direction per layer.
    /// * `node_size` - The size of the smallest nodes (with lod 0).
    /// * `load_distance` - The distance (measured in node sizes) until which to request nodes to be loaded.
    /// * `height` - The height of the terrain.
    pub fn new(
        handle: Handle<Image>,
        lod_count: u32,
        quadtree_size: u32,
        leaf_node_count: f32,
        leaf_node_size: f32,
        load_distance: f32,
        height: f32,
        terrain_size: f32,
        radius: f32,
    ) -> Self {
        Self {
            handle,
            lod_count,
            quadtree_size,
            leaf_node_count,
            leaf_node_size,
            terrain_size,
            radius,
            load_distance,
            _height: height,
            _height_under_viewer: height / 2.0,
            data: Array3::default((
                SIDE_COUNT as usize * lod_count as usize,
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
            view_config.quadtree_handle.clone(),
            config.lod_count,
            view_config.quadtree_size,
            config.leaf_node_count,
            config.leaf_node_size,
            view_config.load_distance,
            config.height,
            config.terrain_size,
            config.radius,
        )
    }

    #[inline]
    fn node_count(&self, lod: u32) -> f32 {
        self.leaf_node_count / (1 << lod) as f32
    }

    #[inline]
    fn node_size(&self, lod: u32) -> f32 {
        self.leaf_node_size / (1 << lod) as f32
    }

    fn origin(&self, quadtree_s2: S2Coordinate, lod: u32) -> UVec2 {
        let origin_node_coordinate = quadtree_s2.st * self.node_count(lod);
        let max_offset = self.node_count(lod).ceil() - self.quadtree_size as f32;

        let quadtree_origin = (origin_node_coordinate - 0.5 * self.quadtree_size as f32)
            .round()
            .clamp(Vec2::splat(0.0), Vec2::splat(max_offset));

        quadtree_origin.as_uvec2()
    }

    fn world_to_local_position(&self, world_position: Vec3) -> Vec3 {
        #[cfg(feature = "spherical")]
        return world_position / self.radius;

        #[cfg(not(feature = "spherical"))]
        return world_position / self.terrain_size;
    }

    pub(crate) fn compute_requests(&mut self, view_world_position: Vec3) {
        let view_local_position = self.world_to_local_position(view_world_position);
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
        (viewer_position / quadtree.node_size as f32).as_uvec2() % quadtree.quadtree_size;

    let node = &quadtree.data[[0, coordinate.y as usize, coordinate.x as usize]];
    let atlas_coords = (viewer_position / quadtree.node_size as f32) % 1.0;

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
