use crate::node_atlas::NodeAtlas;
use crate::terrain::TerrainConfig;
use bevy::{
    asset::{HandleId, LoadState},
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_inspector_egui::Inspectable;
use itertools::iproduct;
use lru::LruCache;

/// Marks a camera as the viewer of the terrain.
/// The view distance is a multiplier, which increases the amount of loaded nodes.
#[derive(Component, Inspectable)]
pub struct ViewDistance {
    #[inspectable(min = 1.0)]
    pub view_distance: f32,
}

impl Default for ViewDistance {
    fn default() -> Self {
        Self { view_distance: 4.0 }
    }
}

#[derive(Clone, Copy)]
pub struct Viewer {
    pub(crate) position: Vec2,
    pub(crate) view_distance: f32,
}

pub(crate) struct NodeData {
    pub(crate) id: u32,
    pub(crate) atlas_index: u16,
    pub(crate) height_map: Handle<Image>,
}

impl NodeData {
    pub(crate) fn load(
        id: u32,
        asset_server: &AssetServer,
        load_statuses: &mut HashMap<u32, LoadStatus>,
        handle_mapping: &mut HashMap<HandleId, u32>,
    ) -> Self {
        let height_map: Handle<Image> = asset_server.load(&format!("output/{}.png", id));

        let status = if asset_server.get_load_state(height_map.clone()) == LoadState::Loaded {
            LoadStatus { finished: true }
        } else {
            handle_mapping.insert(height_map.id, id);
            LoadStatus::default()
        };

        load_statuses.insert(id, status);

        Self {
            id,
            atlas_index: NodeAtlas::INACTIVE_ID,
            height_map,
        }
    }
}

#[derive(Default)]
pub(crate) struct LoadStatus {
    pub(crate) finished: bool,
}

/// Stores all information about the current nodes.
#[derive(Component)]
pub struct Nodes {
    /// Maps the id of an asset to the corresponding node id.
    pub(crate) handle_mapping: HashMap<HandleId, u32>,
    /// Statuses of all currently loading nodes.
    pub(crate) load_statuses: HashMap<u32, LoadStatus>,
    /// Stores the currently loading nodes.
    pub(crate) loading_nodes: HashMap<u32, NodeData>,
    /// Stores the currently active nodes.
    pub(crate) active_nodes: HashMap<u32, NodeData>,
    /// Caches the recently deactivated nodes.
    pub(crate) inactive_nodes: LruCache<u32, NodeData>,
}

impl Nodes {
    pub(crate) fn new(cache_size: usize) -> Self {
        Self {
            handle_mapping: Default::default(),
            load_statuses: Default::default(),
            loading_nodes: Default::default(),
            active_nodes: HashMap::default(),
            inactive_nodes: LruCache::new(cache_size),
        }
    }
}

#[derive(Component)]
pub struct TreeUpdate {
    /// Newly activated nodes since last traversal.
    pub(crate) activated_nodes: HashSet<u32>,
    /// Nodes that are no longer required and should be deactivated.
    pub(crate) nodes_to_deactivate: Vec<u32>,
    /// Nodes that are required and should be loaded and scheduled for activation.
    pub(crate) nodes_to_activate: Vec<u32>,
}

impl TreeUpdate {
    pub(crate) fn new(config: &TerrainConfig) -> Self {
        let lod = config.lod_count - 1;

        let nodes_to_activate = config
            .area_iter()
            .map(|(x, y)| TerrainConfig::node_id(lod, x, y))
            .collect();

        Self {
            activated_nodes: Default::default(),
            nodes_to_deactivate: vec![],
            nodes_to_activate,
        }
    }
}

#[derive(PartialOrd, PartialEq)]
enum NodeState {
    Nonexisting,
    Inactive,
    Loading,
    Active,
}

struct TreeNode {
    id: u32,
    state: NodeState,
    position: Vec2,
    size: f32,
    children: Vec<TreeNode>,
}

impl TreeNode {
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

    fn traverse(&mut self, tree_update: &mut TreeUpdate, viewer: Viewer) {
        // check whether the node has been activated since the last traversal and update it accordingly
        if self.state == NodeState::Loading && tree_update.activated_nodes.contains(&self.id) {
            self.state = NodeState::Active;
        }

        // load a rectangle of nodes around the viewer
        let distance = viewer.position - self.position - Vec2::splat(self.size / 2.0);
        let should_be_active = distance.abs().max_element() < viewer.view_distance * self.size;

        // update the state and determine whether to travers the children
        let traverse_children = match (should_be_active, &self.state) {
            (_, NodeState::Nonexisting) => false,  // does not have children
            (false, NodeState::Inactive) => false, // can't have active children
            (false, NodeState::Loading) => true, // Todo: should this be ignored? cancel into cache
            (false, NodeState::Active) => {
                tree_update.nodes_to_deactivate.push(self.id);
                self.state = NodeState::Inactive;
                true
            }
            (true, NodeState::Inactive) => {
                tree_update.nodes_to_activate.push(self.id);
                self.state = NodeState::Loading;
                true
            }
            (true, NodeState::Loading) => true,
            (true, NodeState::Active) => true,
        };

        // let traverse_children = true;

        if traverse_children {
            for child in &mut self.children {
                child.traverse(tree_update, viewer);
            }
        }
    }
}

#[derive(Component)]
pub struct Quadtree {
    /// The children of the root nodes.
    /// Root nodes stay always loaded, so they don't need to be traversed.
    nodes: Vec<TreeNode>,
}

impl Quadtree {
    pub(crate) fn new(config: &TerrainConfig) -> Self {
        let lod = config.lod_count - 1;

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

        Self { nodes }
    }

    pub(crate) fn traverse(&mut self, tree_update: &mut TreeUpdate, viewer: Viewer) {
        for node in &mut self.nodes {
            node.traverse(tree_update, viewer);
        }
    }
}
