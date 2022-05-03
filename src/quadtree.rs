use crate::{config::TerrainConfig, node_atlas::NodeAtlas};
use bevy::{
    asset::{HandleId, LoadState},
    math::Vec3Swizzles,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    utils::{HashMap, HashSet},
};
use itertools::iproduct;
use lru::LruCache;
use std::mem;

/// Marks a camera as the viewer of the terrain.
/// The view distance is a multiplier, which increases the amount of loaded nodes.
#[derive(Component)]
pub struct ViewDistance {
    // #[inspectable(min = 1.0)]
    pub view_distance: f32,
}

impl Default for ViewDistance {
    fn default() -> Self {
        Self { view_distance: 8.0 }
    }
}

#[derive(Clone, Copy)]
pub struct Viewer {
    pub(crate) position: Vec2,
    pub(crate) view_distance: f32,
}

#[derive(Clone)]
pub struct NodeData {
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

    fn traverse(&mut self, quadtree: &mut Quadtree, viewer: Viewer) {
        // check whether the node has been activated since the last traversal and update it accordingly
        if self.state == NodeState::Loading && quadtree.activated_nodes.contains(&self.id) {
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

        // let traverse_children = true;

        if traverse_children {
            for child in &mut self.children {
                child.traverse(quadtree, viewer);
            }
        }
    }
}

#[derive(Component)]
pub struct Quadtree {
    /// The children of the root nodes.
    /// Root nodes stay always loaded, so they don't need to be traversed.
    nodes: Vec<TreeNode>,
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
    /// Newly activated nodes since last traversal.
    pub(crate) activated_nodes: HashSet<u32>,
    /// Nodes that are no longer required and should be deactivated.
    pub(crate) nodes_to_deactivate: Vec<u32>,
    /// Nodes that are required and should be loaded and scheduled for activation.
    pub(crate) nodes_to_activate: Vec<u32>,
}

impl Quadtree {
    pub(crate) fn new(config: &TerrainConfig, cache_size: usize) -> Self {
        let lod = config.lod_count - 1;

        let nodes_to_activate = config
            .area_iter()
            .map(|(x, y)| TerrainConfig::node_id(lod, x, y))
            .collect();

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
            handle_mapping: default(),
            load_statuses: default(),
            loading_nodes: default(),
            active_nodes: default(),
            inactive_nodes: LruCache::new(cache_size),
            activated_nodes: default(),
            nodes_to_deactivate: default(),
            nodes_to_activate,
        }
    }

    pub(crate) fn traverse(&mut self, viewer: Viewer) {
        let mut nodes = mem::take(&mut self.nodes);

        for node in &mut nodes {
            node.traverse(self, viewer);
        }

        self.nodes = nodes;
    }
}

/// Traverses all quadtrees and generates a new tree update.
pub fn traverse_quadtree(
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

/// Updates the node atlas according to the corresponding tree update and the load statuses.
pub fn update_nodes(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<(&mut Quadtree, &mut NodeAtlas)>,
) {
    for (mut quadtree, mut node_atlas) in terrain_query.iter_mut() {
        let Quadtree {
            ref mut handle_mapping,
            ref mut load_statuses,
            ref mut loading_nodes,
            ref mut inactive_nodes,
            ref mut active_nodes,
            ref mut activated_nodes,
            ref mut nodes_to_activate,
            ref mut nodes_to_deactivate,
            ..
        } = quadtree.as_mut();

        // clear the previously activated nodes
        activated_nodes.clear();

        let mut activation_queue = Vec::new();
        let deactivation_queue = mem::take(nodes_to_deactivate);

        // load required nodes from cache or disk
        for id in mem::take(nodes_to_activate) {
            if let Some(node) = inactive_nodes.pop(&id) {
                // queue cached node for activation
                activation_queue.push(node);
            } else {
                // load node before activation
                loading_nodes.insert(
                    id,
                    NodeData::load(id, &asset_server, load_statuses, handle_mapping),
                );
            };
        }

        // queue all nodes, that have finished loading, for activation
        load_statuses.retain(|&id, status| {
            if status.finished {
                activation_queue.push(loading_nodes.remove(&id).unwrap());
            }

            !status.finished
        });

        // deactivate all no longer required nodes
        for id in deactivation_queue {
            let mut node = active_nodes.remove(&id).unwrap();
            node_atlas.deactivate_node(&mut node);
            inactive_nodes.put(id, node);
        }

        // activate as all nodes ready for activation
        for mut node in activation_queue {
            node_atlas.activate_node(&mut node);
            activated_nodes.insert(node.id);
            active_nodes.insert(node.id, node);
        }
    }
}

/// Updates the load status of a node for all of it newly loaded assets.
pub fn update_load_status(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut terrain_query: Query<&mut Quadtree>,
) {
    for event in asset_events.iter() {
        if let AssetEvent::Created { handle } = event {
            for mut quadtree in terrain_query.iter_mut() {
                if let Some(id) = quadtree.handle_mapping.remove(&handle.id) {
                    let image = images.get_mut(handle).unwrap();

                    image.texture_descriptor.format = TextureFormat::R16Unorm;
                    image.texture_descriptor.usage = TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST
                        | TextureUsages::TEXTURE_BINDING;
                    let status = quadtree.load_statuses.get_mut(&id).unwrap();
                    status.finished = true;
                    break;
                }
            }
        }
    }
}
