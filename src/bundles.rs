use crate::node_atlas::NodeAtlas;
use crate::render::terrain_data::TerrainData;
use crate::render::tile::Tile;
use crate::{
    quadtree::{Nodes, Quadtree, TreeUpdate},
    terrain::TerrainConfig,
    QuadtreeUpdate, TerrainDebugInfo,
};
use bevy::{prelude::*, render::primitives::Aabb};

#[derive(Bundle)]
pub struct TerrainBundle {
    terrain_config: TerrainConfig,
    quadtree: Quadtree,
    tree_update: TreeUpdate,
    nodes: Nodes,
    node_atlas: NodeAtlas,
    quadtree_update: QuadtreeUpdate,
    mesh: Handle<Mesh>,
    terrain_data: Handle<TerrainData>,

    terrain_debug_info: TerrainDebugInfo,

    transform: Transform,
    global_transform: GlobalTransform,
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
    aabb: Aabb,
}

impl TerrainBundle {
    pub fn new(
        config: TerrainConfig,
        meshes: &mut Assets<Mesh>,
        terrain_data: &mut Assets<TerrainData>,
    ) -> Self {
        Self {
            terrain_config: config.clone(),
            quadtree: Quadtree::new(&config),
            tree_update: TreeUpdate::new(&config),
            nodes: Nodes::new(16),
            node_atlas: NodeAtlas::new(config.node_atlas_size),
            quadtree_update: QuadtreeUpdate::default(),
            mesh: meshes.add(Tile::new(config.patch_size as u8, true).to_mesh()),
            terrain_data: terrain_data.add(TerrainData { config }),

            terrain_debug_info: TerrainDebugInfo::default(),

            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            aabb: Aabb::from_min_max(Vec3::splat(-10000.0), Vec3::splat(10000.0)),
        }
    }
}
