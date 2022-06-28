use crate::{node_atlas::NodeAtlas, terrain::Terrain, terrain::TerrainConfig};
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
