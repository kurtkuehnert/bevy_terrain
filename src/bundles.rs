use crate::{config::TerrainConfig, node_atlas::NodeAtlas, Terrain};
use bevy::prelude::*;

#[derive(Bundle)]
pub struct TerrainBundle {
    terrain: Terrain,
    node_atlas: NodeAtlas,
    config: TerrainConfig,
    transform: Transform,
    global_transform: GlobalTransform,
}

impl TerrainBundle {
    pub fn new(config: TerrainConfig) -> Self {
        Self {
            terrain: Terrain,
            node_atlas: NodeAtlas::new(&config),
            config,
            transform: default(),
            global_transform: default(),
        }
    }
}
