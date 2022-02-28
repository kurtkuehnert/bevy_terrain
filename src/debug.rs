use crate::{node_atlas::NodeAtlas, quadtree::Quadtree};
use bevy::prelude::*;

#[derive(Default, Component)]
pub struct TerrainDebugInfo {
    available_ids_len: usize,
    handle_mapping_len: usize,
    load_statuses_len: usize,
    loading_nodes_len: usize,
    active_nodes_len: usize,
    inactive_nodes_len: usize,
}

pub fn info(mut terrain_query: Query<(&mut TerrainDebugInfo, &NodeAtlas, &Quadtree)>) {
    for (mut debug_info, node_atlas, quadtree) in terrain_query.iter_mut() {
        debug_info.available_ids_len = node_atlas.available_ids.len();
        debug_info.handle_mapping_len = quadtree.handle_mapping.len();
        debug_info.load_statuses_len = quadtree.load_statuses.len();
        debug_info.loading_nodes_len = quadtree.loading_nodes.len();
        debug_info.active_nodes_len = quadtree.active_nodes.len();
        debug_info.inactive_nodes_len = quadtree.inactive_nodes.len();
    }
}
