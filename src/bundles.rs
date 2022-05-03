use crate::{config::TerrainConfig, node_atlas::NodeAtlas, quadtree::Quadtree};
use bevy::prelude::*;

#[derive(Bundle)]
pub struct TerrainBundle {
    config: TerrainConfig,
    quadtree: Quadtree,
    node_atlas: NodeAtlas,
    transform: Transform,
    global_transform: GlobalTransform,
}

impl TerrainBundle {
    pub fn new(config: TerrainConfig) -> Self {
        Self {
            config: config.clone(),
            quadtree: Quadtree::new(&config, 16),
            node_atlas: NodeAtlas::new(&config),
            transform: default(),
            global_transform: default(),
        }
    }
}
