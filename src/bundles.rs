use crate::{config::TerrainConfig, node_atlas::NodeAtlas, quadtree::Quadtree};
use bevy::prelude::*;

#[derive(Bundle)]
pub struct TerrainBundle {
    quadtree: Quadtree,
    node_atlas: NodeAtlas,
    config: TerrainConfig,
    transform: Transform,
    global_transform: GlobalTransform,
}

impl TerrainBundle {
    pub fn new(config: TerrainConfig) -> Self {
        Self {
            quadtree: Quadtree::new(&config),
            node_atlas: NodeAtlas::new(&config, 16),
            config,
            transform: default(),
            global_transform: default(),
        }
    }
}
