use crate::node_atlas::NodeAtlas;
use crate::quadtree::Nodes;
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

#[derive(Default, Component, Inspectable)]
pub struct TerrainDebugInfo {
    available_ids_len: usize,
    handle_mapping_len: usize,
    load_statuses_len: usize,
    loading_nodes_len: usize,
    active_nodes_len: usize,
    inactive_nodes_len: usize,
}

pub fn info(mut terrain_query: Query<(&mut TerrainDebugInfo, &NodeAtlas, &Nodes)>) {
    for (mut debug_info, node_atlas, nodes) in terrain_query.iter_mut() {
        debug_info.available_ids_len = node_atlas.available_ids.len();
        debug_info.handle_mapping_len = nodes.handle_mapping.len();
        debug_info.load_statuses_len = nodes.load_statuses.len();
        debug_info.loading_nodes_len = nodes.loading_nodes.len();
        debug_info.active_nodes_len = nodes.active_nodes.len();
        debug_info.inactive_nodes_len = nodes.inactive_nodes.len();
    }
}
