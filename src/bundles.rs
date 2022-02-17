use crate::{
    config::TerrainConfig, node_atlas::NodeAtlas, quadtree::Quadtree,
    render::terrain_data::TerrainData, TerrainDebugInfo,
};
use bevy::prelude::*;

#[derive(Bundle)]
pub struct TerrainBundle {
    terrain_config: TerrainConfig,
    quadtree: Quadtree,
    node_atlas: NodeAtlas,
    terrain_data: Handle<TerrainData>,
    terrain_debug_info: TerrainDebugInfo,
    transform: Transform,
    global_transform: GlobalTransform,
}

impl TerrainBundle {
    pub fn new(config: TerrainConfig, terrain_data: &mut Assets<TerrainData>) -> Self {
        Self {
            terrain_config: config.clone(),
            quadtree: Quadtree::new(&config, 16),
            node_atlas: NodeAtlas::new(&config),
            terrain_data: terrain_data.add(TerrainData { config }),
            terrain_debug_info: TerrainDebugInfo::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
