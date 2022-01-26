use crate::terrain::TerrainConfig;
use bevy::{
    asset::{HandleId, LoadState},
    math::Vec3Swizzles,
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_inspector_egui::Inspectable;
use itertools::iproduct;
use lru::LruCache;
use std::mem;

/// Marks a camera as the viewer of the terrain.
/// The view distance is a multiplier, which increases the amount of loaded nodes.
#[derive(Component, Inspectable)]
pub struct ViewDistance {
    #[inspectable(min = 1.0)]
    pub view_distance: f32,
}

impl Default for ViewDistance {
    fn default() -> Self {
        Self { view_distance: 1.0 }
    }
}

#[derive(Clone, Copy)]
pub struct Viewer {
    position: Vec2,
    view_distance: f32,
}

pub(crate) struct NodeData {
    id: u32,
    atlas_id: u16,
    height_map: Handle<Image>,
}

impl NodeData {
    fn load(
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
            atlas_id: NodeAtlas::INACTIVE_ID,
            height_map,
        }
    }
}

#[derive(Default)]
pub(crate) struct LoadStatus {
    finished: bool,
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
    pub fn new(cache_size: usize) -> Self {
        Self {
            handle_mapping: Default::default(),
            load_statuses: Default::default(),
            loading_nodes: Default::default(),
            active_nodes: HashMap::default(),
            inactive_nodes: LruCache::new(cache_size),
        }
    }
}

pub(crate) struct NodeUpdate {
    pub(crate) node_id: u32,
    pub(crate) atlas_id: u16,
}

/// Maps the assets to the corresponding active nodes and tracks the node updates.
#[derive(Component)]
pub struct NodeAtlas {
    pub(crate) height_maps: Vec<Handle<Image>>,
    pub(crate) node_updates: Vec<NodeUpdate>,
    pub(crate) available_ids: Vec<usize>,
}

impl NodeAtlas {
    pub(crate) const INVALID_ID: u16 = u16::MAX;
    pub(crate) const INACTIVE_ID: u16 = u16::MAX - 1;

    pub fn new(atlas_size: usize) -> Self {
        Self {
            height_maps: vec![Handle::default(); atlas_size],
            node_updates: vec![],
            available_ids: (0..atlas_size).collect(),
        }
    }

    fn add_node(&mut self, node: &mut NodeData) {
        let atlas_id = self.available_ids.pop().expect("Out of atlas ids.");

        self.height_maps[atlas_id] = node.height_map.as_weak();
        node.atlas_id = atlas_id as u16;

        self.node_updates.push(NodeUpdate {
            node_id: node.id,
            atlas_id: node.atlas_id,
        });
    }

    fn remove_node(&mut self, node: &mut NodeData) {
        self.available_ids.push(node.atlas_id as usize);

        node.atlas_id = Self::INACTIVE_ID;

        self.node_updates.push(NodeUpdate {
            node_id: node.id,
            atlas_id: node.atlas_id,
        });
    }
}

#[derive(Component)]
pub struct TreeUpdate {
    /// Newly activated nodes since last traversal.
    activated_nodes: HashSet<u32>,
    /// Nodes that are no longer required and should be deactivated.
    nodes_to_deactivate: Vec<u32>,
    /// Nodes that are required and should be loaded and scheduled for activation.
    nodes_to_activate: Vec<u32>,
}

impl TreeUpdate {
    pub fn new(config: &TerrainConfig) -> Self {
        let lod = config.lod_count - 1;

        let nodes_to_activate = config
            .area_iter()
            .map(|(x, y)| config.node_id(lod, x, y))
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
        let id = config.node_id(lod, x, y);
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
    pub fn new(config: &TerrainConfig) -> Self {
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

    fn traverse(&mut self, tree_update: &mut TreeUpdate, viewer: Viewer) {
        for node in &mut self.nodes {
            node.traverse(tree_update, viewer);
        }
    }
}

/// Traverses all quadtrees and generates a new tree update.
pub fn traverse_quadtree(
    viewer_query: Query<(&GlobalTransform, &ViewDistance), With<Camera>>,
    mut terrain_query: Query<(&GlobalTransform, &mut Quadtree, &mut TreeUpdate)>,
) {
    for (terrain_transform, mut quadtree, mut tree_update) in terrain_query.iter_mut() {
        for (camera_transform, view_distance) in viewer_query.iter() {
            let viewer = Viewer {
                position: (camera_transform.translation - terrain_transform.translation).xz(),
                view_distance: view_distance.view_distance,
            };

            quadtree.traverse(&mut tree_update, viewer);
        }
    }
}

/// Updates the nodes and the node atlas according to the corresponding tree update
/// and the load statuses.
pub fn update_nodes(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<(&mut TreeUpdate, &mut Nodes, &mut NodeAtlas)>,
) {
    for (mut tree_update, mut nodes, mut node_atlas) in terrain_query.iter_mut() {
        let Nodes {
            ref mut handle_mapping,
            ref mut load_statuses,
            ref mut loading_nodes,
            ref mut inactive_nodes,
            ref mut active_nodes,
        } = nodes.as_mut();

        // clear the previously activated nodes
        tree_update.activated_nodes.clear();
        node_atlas.node_updates.clear();

        let mut nodes_to_activate: Vec<NodeData> = Vec::new();

        // load required nodes from cache or disk
        for id in mem::take(&mut tree_update.nodes_to_activate) {
            if let Some(node) = inactive_nodes.pop(&id) {
                // queue cached node for activation
                nodes_to_activate.push(node);
            } else {
                // load node before activation
                loading_nodes.insert(
                    id,
                    NodeData::load(id, &asset_server, load_statuses, handle_mapping),
                );
            };
        }

        // queue all nodes that have finished loading for activation
        load_statuses.retain(|&id, status| {
            if status.finished {
                nodes_to_activate.push(loading_nodes.remove(&id).unwrap());
            }

            !status.finished
        });

        // deactivate all no longer required nodes
        for id in mem::take(&mut tree_update.nodes_to_deactivate) {
            let mut node = active_nodes.remove(&id).unwrap();
            node_atlas.remove_node(&mut node);
            inactive_nodes.put(id, node);
        }

        // activate as many nodes as there are available atlas ids
        for mut node in nodes_to_activate {
            node_atlas.add_node(&mut node);
            tree_update.activated_nodes.insert(node.id);
            active_nodes.insert(node.id, node);
        }
    }
}

/// Updates the load status of a node for all of it newly loaded assets.
pub fn update_load_status(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut terrain_query: Query<&mut Nodes>,
) {
    for event in asset_events.iter() {
        if let AssetEvent::Created { handle } = event {
            for mut nodes in terrain_query.iter_mut() {
                if let Some(id) = nodes.handle_mapping.remove(&handle.id) {
                    let status = nodes.load_statuses.get_mut(&id).unwrap();
                    status.finished = true;
                    break;
                }
            }
        }
    }
}
