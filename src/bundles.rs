use crate::data_structures::node_atlas::NodeAtlas;
use crate::{terrain::Terrain, terrain::TerrainConfig};
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
            node_atlas: NodeAtlas::from_config(&config),
            config,
            transform: default(),
            global_transform: default(),
        }
    }
}
