use crate::{config::TerrainConfig, node_atlas::NodeAtlas, quadtree::Quadtree, TerrainDebugInfo};
use bevy::prelude::*;

#[derive(Bundle)]
pub struct TerrainBundle {
    terrain_config: TerrainConfig,
    quadtree: Quadtree,
    node_atlas: NodeAtlas,
    terrain_debug_info: TerrainDebugInfo,
    transform: Transform,
    global_transform: GlobalTransform,
}

impl TerrainBundle {
    pub fn new(config: TerrainConfig) -> Self {
        Self {
            terrain_config: config.clone(),
            quadtree: Quadtree::new(&config, 16),
            node_atlas: NodeAtlas::new(&config),
            terrain_debug_info: default(),
            transform: default(),
            global_transform: default(),
        }
    }
}
