use crate::pipeline::{TerrainData, TileData};
use crate::quadtree::{NodeAtlas, Nodes};
use crate::terrain::TerrainConfig;
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

pub fn debug(mut terrain_query: Query<(&TerrainConfig, &NodeAtlas, &mut TerrainData)>) {
    for (config, node_atlas, mut terrain_data) in terrain_query.iter_mut() {
        let data: &mut Vec<TileData> = &mut terrain_data.data;

        for update in &node_atlas.node_updates {
            let (lod, x, y) = config.node_position(update.node_id);
            let size = config.node_size(lod);
            let position = UVec2::new(x, y) * size;

            if update.atlas_id == NodeAtlas::INACTIVE_ID {
                data.retain(|item| !(item.position == position && item.size == size));
            } else {
                let color = match lod {
                    0 => Color::RED.into(),
                    1 => Color::BLUE.into(),
                    2 => Color::GREEN.into(),
                    _ => Color::BLACK.into(),
                };
                data.push(TileData {
                    position,
                    size,
                    range: 32.0,
                    color,
                });
            }
        }
    }
}

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
