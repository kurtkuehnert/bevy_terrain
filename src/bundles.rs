use crate::{
    pipeline::TerrainData,
    quadtree::{NodeAtlas, Nodes, Quadtree, TreeUpdate},
    terrain::TerrainConfig,
};
use bevy::{prelude::*, render::primitives::Aabb};

#[derive(Bundle)]
pub struct TerrainBundle {
    terrain_config: TerrainConfig,
    pub quadtree: Quadtree,
    pub tree_update: TreeUpdate,
    pub nodes: Nodes,
    pub node_atlas: NodeAtlas,
    pub transform: Transform,
    pub global_transform: GlobalTransform,

    instance_data: TerrainData,
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
    aabb: Aabb,
}

impl TerrainBundle {
    pub fn new(config: TerrainConfig) -> Self {
        Self {
            quadtree: Quadtree::new(&config),
            tree_update: TreeUpdate::new(&config),
            nodes: Nodes::new(16),
            node_atlas: NodeAtlas::new(64),
            transform: Default::default(),
            global_transform: Default::default(),
            terrain_config: config,

            instance_data: TerrainData::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            aabb: Aabb::from_min_max(Vec3::splat(-10000.0), Vec3::splat(10000.0)),
        }
    }
}
