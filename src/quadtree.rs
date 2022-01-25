use crate::terrain::TerrainConfig;
use bevy::asset::LoadState;
use bevy::{
    asset::HandleId,
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
        Self { view_distance: 2.0 }
    }
}

#[derive(Clone, Copy)]
pub struct Viewer {
    position: Vec2,
    view_distance: f32,
}

struct NodeData {
    id: u16,
    atlas_id: u16,
    height_map: Handle<Image>,
}

impl NodeData {
    fn load(
        id: u16,
        asset_server: &AssetServer,
        load_statuses: &mut HashMap<u16, LoadStatus>,
        handle_mapping: &mut HashMap<HandleId, u16>,
    ) -> Self {
        let config = TerrainConfig::new(128, 3, UVec2::new(2, 2));
        println!("{:?}", config.node_position(id));

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
            atlas_id: u16::MAX,
            height_map,
        }
    }
}

#[derive(Default)]
struct LoadStatus {
    finished: bool,
}

/// Stores all information about the current nodes.
#[derive(Component)]
pub struct Nodes {
    /// Stores the nodes, that are queued for activation.
    activation_queue: Vec<NodeData>, // Todo: consider storing ids here as well
    /// Maps the id of an asset to the corresponding node id.
    handle_mapping: HashMap<HandleId, u16>,
    /// Statuses of all current nodes.
    load_statuses: HashMap<u16, LoadStatus>,
    /// Stores the currently loading nodes.
    loading_nodes: HashMap<u16, NodeData>,
    /// Stores the currently active nodes.
    active_nodes: HashMap<u16, NodeData>,
    /// Caches recently unloaded nodes.
    inactive_nodes: LruCache<u16, NodeData>,
}

impl Nodes {
    pub fn new(atlas_size: u16, cache_size: usize) -> Self {
        let inactive_nodes = LruCache::new(cache_size);
        let mut active_nodes = HashMap::default();

        for atlas_id in 0..atlas_size {
            active_nodes.insert(
                atlas_id,
                NodeData {
                    id: 0,
                    atlas_id,
                    height_map: Default::default(),
                },
            );
        }

        Self {
            activation_queue: Default::default(),
            handle_mapping: Default::default(),
            load_statuses: Default::default(),
            loading_nodes: Default::default(),
            active_nodes,
            inactive_nodes,
        }
    }
}

struct NodeUpdate {
    node_pos: u32,
    atlas_id: u16,
}

#[derive(Default, Component)]
pub struct NodeAtlas {
    height_maps: Vec<Handle<Image>>,
    node_updates: Vec<NodeUpdate>,
    available_ids: Vec<u16>,
}

impl NodeAtlas {
    const INVALID_ID: u16 = u16::MAX;
    const INACTIVE_ID: u16 = u16::MAX;

    fn add_node(&mut self, node: &mut NodeData) {
        let atlas_id = self.available_ids.pop().unwrap();

        self.height_maps[atlas_id as usize] = node.height_map.as_weak();

        self.node_updates.push(NodeUpdate {
            node_pos: node.id as u32,
            atlas_id: node.atlas_id,
        });
    }

    fn remove_node(&mut self, node: &mut NodeData) {
        self.available_ids.push(node.atlas_id);

        self.node_updates.push(NodeUpdate {
            node_pos: node.id as u32,
            atlas_id: Self::INACTIVE_ID,
        });
    }
}

#[derive(Default, Component)]
pub struct TreeUpdate {
    /// Nodes that are no longer required and should be deactivated.
    nodes_to_deactivate: Vec<u16>,
    /// Nodes that are required and should be loaded and scheduled for activation.
    nodes_to_load: Vec<u16>,
    /// Newly activated nodes since last traversal.
    activated_nodes: HashSet<u16>,
}

#[derive(PartialOrd, PartialEq)]
enum NodeState {
    Nonexisting,
    Inactive,
    Loading,
    Active,
}

struct Node {
    id: u16,
    root: bool,
    position: Vec2,
    size: f32,
    state: NodeState,
    children: Vec<Node>,
}

impl Node {
    fn new(config: &TerrainConfig, lod: u32, x: u32, y: u32) -> Self {
        let id = config.node_id(lod, x, y);
        let size = config.node_size(lod) as f32;
        let position = Vec2::new(x as f32 * size, y as f32 * size);
        let root = lod == config.lod_count - 1;

        let children = match lod {
            0 => Vec::new(),
            _ => iproduct!(0..2, 0..2)
                .map(|(ox, oy)| Node::new(config, lod - 1, 2 * x + ox, 2 * y + oy))
                .collect(),
        };

        Self {
            id,
            root,
            position,
            size,
            state: NodeState::Inactive,
            children,
        }
    }

    fn traverse(&mut self, tree_update: &mut TreeUpdate, viewer: Viewer) {
        // check whether the node has been activated since the last traversal and update it accordingly
        if self.state == NodeState::Loading && tree_update.activated_nodes.contains(&self.id) {
            self.state = NodeState::Active;
        }

        let should_be_active = if self.root {
            true
        } else {
            // load a rectangle of nodes around the viewer
            let distance = (viewer.position - self.position).abs().min_element();
            distance < viewer.view_distance * self.size
        };

        // update the state and determine whether to travers the children
        let traverse_children = match (should_be_active, &self.state) {
            (_, NodeState::Nonexisting) => false,  // does not have children
            (false, NodeState::Inactive) => false, // can't have active children
            (false, NodeState::Loading) => true,   // Todo: should this be ignored?
            (false, NodeState::Active) => {
                tree_update.nodes_to_deactivate.push(self.id);
                self.state = NodeState::Inactive;
                true
            }
            (true, NodeState::Inactive) => {
                tree_update.nodes_to_load.push(self.id);
                self.state = NodeState::Loading;
                true
            }
            (true, NodeState::Loading) => true,
            (true, NodeState::Active) => true,
        };

        if traverse_children {
            for child in &mut self.children {
                child.traverse(tree_update, viewer);
            }
        }
    }
}

#[derive(Component)]
pub struct Quadtree {
    root_nodes: Vec<Node>,
}

impl Quadtree {
    pub fn new(config: &TerrainConfig) -> Self {
        let root_nodes = iproduct!(0..config.area_count.x, 0..config.area_count.y)
            .map(|(x, y)| Node::new(config, config.lod_count - 1, x, y))
            .collect();

        Self { root_nodes }
    }

    fn traverse(&mut self, tree_update: &mut TreeUpdate, viewer: Viewer) {
        for node in &mut self.root_nodes {
            node.traverse(tree_update, viewer);
        }
    }
}

pub fn traverse_quadtree(
    viewer_query: Query<(&GlobalTransform, &ViewDistance), With<Camera>>,
    mut terrain_query: Query<(&GlobalTransform, &mut Quadtree, &mut TreeUpdate)>,
) {
    for (terrain_transform, mut quadtree, mut tree_update) in terrain_query.iter_mut() {
        for (camera_transform, view_distance) in viewer_query.iter() {
            let viewer = Viewer {
                position: (camera_transform.translation - terrain_transform.translation).xy(),
                view_distance: view_distance.view_distance,
            };

            quadtree.traverse(&mut tree_update, viewer);
        }
    }
}

pub fn update_nodes(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<(&mut TreeUpdate, &mut Nodes)>,
) {
    for (mut tree_update, mut nodes) in terrain_query.iter_mut() {
        let Nodes {
            ref mut activation_queue,
            ref mut handle_mapping,
            ref mut load_statuses,
            ref mut loading_nodes,
            ref mut inactive_nodes,
            ..
        } = nodes.as_mut();

        // check whether newly required nodes are already cached or have to be loaded
        for id in mem::take(&mut tree_update.nodes_to_load) {
            if let Some(node) = inactive_nodes.pop(&id) {
                // queue cached node for activation
                activation_queue.push(node);
            } else {
                // load node
                loading_nodes.insert(
                    id,
                    NodeData::load(id, &asset_server, load_statuses, handle_mapping),
                );
            };
        }

        // check all nodes, that have finished loading and queue them for activation
        load_statuses.retain(|&id, status| {
            if status.finished {
                let node = loading_nodes.remove(&id).unwrap();
                activation_queue.push(node);
            }

            !status.finished
        });
    }
}

pub fn update_atlas(mut terrain_query: Query<(&mut TreeUpdate, &mut Nodes, &mut NodeAtlas)>) {
    for (mut tree_update, mut nodes, mut node_atlas) in terrain_query.iter_mut() {
        let Nodes {
            ref mut activation_queue,
            ref mut inactive_nodes,
            ref mut active_nodes,
            ..
        } = nodes.as_mut();

        // clear the previously updated nodes
        tree_update.activated_nodes.clear();

        for id in mem::take(&mut tree_update.nodes_to_load) {
            let node = active_nodes.remove(&id).unwrap();

            node_atlas.available_ids.push(node.atlas_id);

            inactive_nodes.put(id, node);
        }

        // replace the old node with the new one
        for mut node in activation_queue.drain(0..node_atlas.available_ids.len()) {
            node_atlas.add_node(&mut node);

            // inform the tree about the update
            tree_update.activated_nodes.insert(node.id);
            active_nodes.insert(node.id, node);
        }
    }
}

pub fn update_load_status(
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut terrain_query: Query<&mut Nodes>,
) {
    for event in asset_events.iter() {
        if let AssetEvent::Created { handle } = event {
            let handle_id = handle.id;

            for mut nodes in terrain_query.iter_mut() {
                if let Some(id) = nodes.handle_mapping.remove(&handle_id) {
                    let status = nodes.load_statuses.get_mut(&id).unwrap();
                    status.finished = true;
                    break;
                }
            }
        }
    }
}

/*

/// Marks a camera as the viewer of the terrain.
/// This is used to select the visible nodes of the quadtree.
/// The view distance is a multiplier, which increases the size of the lod ranges.
#[derive(Component, Inspectable)]
pub struct Viewer {
    #[inspectable(min = 1.0)]
    pub view_distance: f32,
}

impl Default for Viewer {
    fn default() -> Self {
        Self { view_distance: 2.0 }
    }
}

/// Selection of visible nodes.
/// They are divided into dense and sparse nodes.
/// The later have half the resolution of the former.
#[derive(Default)]
struct Selection {
    dense_nodes: Vec<TileData>,
    sparse_nodes: Vec<TileData>,
}

impl Selection {
    fn add_node(&mut self, node: Node, dense: bool) {
        if dense {
            self.dense_nodes.push(node.into());
        } else {
            self.sparse_nodes.push(node.into());
        }
    }
}

/// Node representation used while traversing the quadtree.
/// They get converted into [`TileData`], the Gpu equivalent.
#[derive(Clone, Debug)]
struct Node {
    position: UVec2,
    size: u32,
    lod: u8,
    range: f32,
}

impl From<Node> for TileData {
    fn from(val: Node) -> Self {
        TileData {
            position: val.position,
            size: val.size,
            range: val.range,
            color: Vec4::from(match val.lod {
                0 => Color::YELLOW,
                1 => Color::RED,
                2 => Color::GREEN,
                3 => Color::BLUE,
                4 => Color::YELLOW,
                5 => Color::RED,
                6 => Color::GREEN,
                7 => Color::BLUE,
                _ => Color::BLACK,
            }),
        }
    }
}

impl Node {
    /// Recursively traverses the quadtree, while selecting all currently visible nodes.
    fn select_lod(
        self,
        quadtree: &Quadtree,
        camera_position: Vec3,
        local_to_world: &Mat4,
        frustum: &Frustum,
        selection: &mut Selection,
    ) -> bool {
        // Todo: accurately calculate min and max heights
        let aabb = Aabb::from_min_max(
            Vec3::new(self.position.x as f32, 0.0, self.position.y as f32),
            Vec3::new(
                (self.position.x + self.size) as f32,
                500.0,
                (self.position.y + self.size) as f32,
            ),
        );

        let sphere = Sphere {
            center: camera_position,
            radius: self.range,
        };

        // test, whether the node is inside the current lod range
        if !sphere.intersects_obb(&aabb, local_to_world) {
            return false; // area handled by parent
        }

        // test, whether the node is inside the cameras view frustum
        if !frustum.intersects_obb(&aabb, local_to_world) {
            return true; // area not visible
        }

        // last lod can't be subdivided, so we included it altogether
        if self.lod == 0 {
            selection.add_node(self, true);
            return true; // area covered by the current node, which can't be subdivided
        }

        let lod = self.lod - 1;
        let range = quadtree.lod_ranges[lod as usize];
        let sphere = Sphere {
            center: camera_position,
            radius: range,
        };

        // test, whether the node is inside the next smaller lod range
        if !sphere.intersects_obb(&aabb, local_to_world) {
            selection.add_node(self, true);
            return true; // area covered by the current node, which isn't subdivided
        }

        let size = self.size >> 1;

        // if this loop is reached the node has to be subdivided into four children
        for (x, y) in iproduct!(0..2, 0..2) {
            let child = Node {
                position: self.position + UVec2::new(x * size, y * size),
                size,
                lod,
                range,
            };

            // test, whether the child is successfully selected
            if !child.clone().select_lod(
                quadtree,
                camera_position,
                local_to_world,
                frustum,
                selection,
            ) {
                // the child was not selected yet, so it has to be selected as a part of its parent,
                // with a sparse grid
                selection.add_node(child, false);
            }
        }

        true // area covered by the child nodes
    }
}

/// The quadtree responsible for the [`Node`] selection.
/// Every frame the tree is traversed and all visible nodes are selected.
#[derive(Component, Inspectable)]
pub struct Quadtree {
    node_size: u32,
    lod_count: u8,
    lod_ranges: Vec<f32>,
}

impl Default for Quadtree {
    fn default() -> Self {
        Self {
            node_size: 4,
            lod_count: 4,
            lod_ranges: Vec::new(),
        }
    }
}

pub fn traverse_quadtree(
    viewer_query: Query<(&GlobalTransform, &Frustum), (With<Viewer>, With<Camera>)>,
    mut quadtree_query: Query<(&Children, &Quadtree, &GlobalTransform)>,
    mut terrain_query: Query<&mut TerrainData>,
) {
    for (children, quadtree, terrain_transform) in quadtree_query.iter_mut() {
        for (camera_transform, frustum) in viewer_query.iter() {
            let lod = quadtree.lod_count - 1;

            let root = Node {
                position: UVec2::ZERO,
                size: quadtree.node_size * (1 << lod as u32),
                lod,
                range: quadtree.lod_ranges[lod as usize],
            };

            let mut selection = Selection::default();

            root.select_lod(
                quadtree,
                camera_transform.translation,
                &terrain_transform.compute_matrix(),
                frustum,
                &mut selection,
            );

            // println!(
            //     "{} {}",
            //     selection.dense_nodes.len(),
            //     selection.sparse_nodes.len()
            // );

            for &child in children.iter() {
                let mut instance_data = terrain_query.get_mut(child).unwrap();

                if instance_data.sparse {
                    instance_data.data = take(&mut selection.sparse_nodes);
                } else {
                    instance_data.data = take(&mut selection.dense_nodes);
                }
            }
        }
    }
}

pub fn update_quadtree_on_change(
    mut meshes: ResMut<Assets<Mesh>>,
    viewer_query: Query<&Viewer>,
    mut quadtree_query: Query<
        (&Children, &mut Quadtree, &QuadtreeDescriptor),
        Changed<QuadtreeDescriptor>,
    >,
    terrain_query: Query<(&TerrainData, &Handle<Mesh>)>,
) {
    for (children, mut quadtree, quadtree_descriptor) in quadtree_query.iter_mut() {
        quadtree.node_size = quadtree_descriptor.node_size as u32;
        quadtree.lod_count = quadtree_descriptor.lod_count;
        update_view_distance(viewer_query.iter(), &mut quadtree);

        for &child in children.iter() {
            let (terrain_data, mesh) = terrain_query.get(child).unwrap();

            let mesh = meshes
                .get_mut(mesh.clone())
                .expect("Instance mesh not initialized.");

            let size = if terrain_data.sparse {
                quadtree.node_size / 2
            } else {
                quadtree.node_size
            } as u8;

            *mesh = Tile::new(size, quadtree_descriptor.wireframe).to_mesh();
        }
    }
}

pub fn update_view_distance_on_change(
    mut quadtree_query: Query<&mut Quadtree>,
    viewer_query: Query<&Viewer, Changed<Viewer>>,
) {
    for mut quadtree in quadtree_query.iter_mut() {
        update_view_distance(viewer_query.iter(), &mut quadtree);
    }
}

fn update_view_distance<'a>(viewer: impl Iterator<Item = &'a Viewer>, quadtree: &mut Quadtree) {
    for viewer in viewer {
        quadtree.lod_ranges = (0..quadtree.lod_count)
            .map(|lod| quadtree.node_size as f32 * viewer.view_distance * (2 << lod as u32) as f32)
            .collect();

        // println!("{:?}", quadtree.lod_ranges);
    }
}


 */
