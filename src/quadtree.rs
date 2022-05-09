use crate::{
    config::{NodeId, TerrainConfig},
    viewer::{ViewDistance, Viewer},
};
use bevy::{
    core::{Pod, Zeroable},
    math::Vec3Swizzles,
    prelude::*,
    utils::HashSet,
};
use itertools::iproduct;
use std::mem;

/// An update to the [`GpuQuadtree`](crate::render::gpu_quadtree::GpuQuadtree).
/// This update is created whenever a node becomes activated/deactivated by
/// the [`NodeAtlas`](crate::node_atlas::NodeAtlas).
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
pub struct NodeUpdate {
    /// The id of the updated node.
    pub(crate) node_id: NodeId,
    /// The new atlas index of the node.
    pub(crate) atlas_index: u32, // u16 not supported by std 140
}

/// The state of a [`TreeNode`] inside the quadtree.
#[derive(PartialOrd, PartialEq)]
enum NodeState {
    /// This node does not exist. Useful for sparse terrains, which are not rectangular in shape.
    Nonexistent,
    /// The node is not part of the [`NodeAtlas`](crate::node_atlas::NodeAtlas) and therefore
    /// not available for rendering. It may or may not be loaded.
    Inactive,
    /// The node is scheduled for activation, but was not confirmed to be fully loaded and thus
    /// part of the [`NodeAtlas`](crate::node_atlas::NodeAtlas) yet.
    Loading,
    /// The node is fully loaded and part of the [`NodeAtlas`](crate::node_atlas::NodeAtlas).
    Active,
}

/// All information required by the [`Quadtree`] traversal algorithm to determine
/// the [`NodeState`] the node should be in.
struct TreeNode {
    id: NodeId,
    state: NodeState,
    position: Vec2,
    size: f32,
    children: Vec<TreeNode>,
}

impl TreeNode {
    /// Creates a new node and its children according to the supplied [`TerrainConfig`].
    fn new(config: &TerrainConfig, lod: u32, x: u32, y: u32) -> Self {
        let id = TerrainConfig::node_id(lod, x, y);
        let state = NodeState::Inactive;
        let size = config.node_size(lod) as f32;
        let position = Vec2::new(x as f32 * size, y as f32 * size);

        let children = match lod {
            0 => Vec::new(),
            _ => iproduct!(0..2, 0..2)
                .map(|(ox, oy)| TreeNode::new(config, lod - 1, 2 * x + ox, 2 * y + oy))
                .collect(),
        };

        Self {
            id,
            state,
            position,
            size,
            children,
        }
    }

    /// Traverses the node and its children to update their [`NodeState`]s and
    /// marking nodes to activate/deactivate.
    fn traverse(&mut self, quadtree: &mut Quadtree, viewer: Viewer) {
        // check whether the node has been activated since the last traversal and update it accordingly
        if self.state == NodeState::Loading && quadtree.nodes_activated.contains(&self.id) {
            self.state = NodeState::Active;
        }

        // load a rectangle of nodes around the viewer
        let distance = viewer.position - self.position - Vec2::splat(self.size / 2.0);
        let should_be_active = distance.abs().max_element() < viewer.view_distance * self.size;

        // update the state and determine whether to travers the children
        let traverse_children = match (should_be_active, &self.state) {
            (_, NodeState::Nonexistent) => false,  // does not have children
            (false, NodeState::Inactive) => false, // can't have active children
            (false, NodeState::Loading) => true, // Todo: should this be ignored? cancel into cache
            (false, NodeState::Active) => {
                quadtree.nodes_to_deactivate.push(self.id);
                self.state = NodeState::Inactive;
                true
            }
            (true, NodeState::Inactive) => {
                quadtree.nodes_to_activate.push(self.id);
                self.state = NodeState::Loading;
                true
            }
            (true, NodeState::Loading) => true,
            (true, NodeState::Active) => true,
        };

        if traverse_children {
            for child in &mut self.children {
                child.traverse(quadtree, viewer);
            }
        }
    }
}

/// Stores all of the tree nodes of a terrain and decides which nodes to activate/deactivate.
/// Additionally it tracks all [`NodeUpdate`]s, which are send to the GPU by the
/// [`GpuQuadtree`](crate::render::gpu_quadtree::GpuQuadtree).
#[derive(Component)]
pub struct Quadtree {
    /// The children of the root nodes.
    /// Root nodes stay always loaded, so they don't need to be traversed.
    nodes: Vec<TreeNode>,
    /// Newly activated nodes since last traversal.
    pub(crate) nodes_activated: HashSet<NodeId>,
    /// Nodes that are no longer required and should be deactivated.
    pub(crate) nodes_to_deactivate: Vec<NodeId>,
    /// Nodes that are required and should be loaded and scheduled for activation.
    pub(crate) nodes_to_activate: Vec<NodeId>,
    /// All newly generate updates of nodes, which were activated or deactivated.
    pub(crate) node_updates: Vec<Vec<NodeUpdate>>,
}

impl Quadtree {
    /// Creates a new quadtree based on the supplied [`TerrainConfig`].
    pub(crate) fn new(config: &TerrainConfig) -> Self {
        let lod = config.lod_count - 1;

        // activate root nodes
        let nodes_to_activate = config
            .area_iter()
            .map(|(x, y)| TerrainConfig::node_id(lod, x, y))
            .collect();

        // create all nodes of the tree
        let nodes = match lod {
            0 => Vec::new(),
            _ => config
                .area_iter()
                .flat_map(|(x, y)| {
                    iproduct!(0..2, 0..2)
                        .map(move |(ox, oy)| TreeNode::new(config, lod - 1, 2 * x + ox, 2 * y + oy))
                })
                .collect(),
        };

        Self {
            nodes,
            nodes_activated: default(),
            nodes_to_deactivate: default(),
            nodes_to_activate,
            node_updates: vec![default(); config.lod_count as usize],
        }
    }

    /// Traverses the quadtree and marks all nodes to activate/deactivate.
    pub(crate) fn traverse(&mut self, viewer: Viewer) {
        let mut nodes = mem::take(&mut self.nodes);

        for node in &mut nodes {
            node.traverse(self, viewer);
        }

        self.nodes = nodes;

        self.nodes_activated.clear();
    }
}

/// Traverses all quadtrees and marks all nodes to activate/deactivate.
pub(crate) fn traverse_quadtree(
    viewer_query: Query<(&GlobalTransform, &ViewDistance), With<Camera>>,
    mut terrain_query: Query<(&GlobalTransform, &mut Quadtree)>,
) {
    for (terrain_transform, mut quadtree) in terrain_query.iter_mut() {
        for (camera_transform, view_distance) in viewer_query.iter() {
            let viewer = Viewer {
                position: (camera_transform.translation - terrain_transform.translation).xz(),
                view_distance: view_distance.view_distance,
            };

            quadtree.traverse(viewer);
        }
    }
}
