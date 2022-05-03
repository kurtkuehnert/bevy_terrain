use crate::{config::TerrainConfig, quadtree::NodeData};
use bevy::prelude::*;

use std::collections::VecDeque;

/// Maps the assets to the corresponding active nodes and tracks the node updates.
#[derive(Component)]
pub struct NodeAtlas {
    pub(crate) available_indices: VecDeque<u16>,
    pub(crate) activated_nodes: Vec<(u16, NodeData)>,
}

impl NodeAtlas {
    // pub(crate) const NONEXISTENT_ID: u16 = u16::MAX;
    pub(crate) const INACTIVE_ID: u16 = u16::MAX - 1;

    pub fn new(config: &TerrainConfig) -> Self {
        Self {
            available_indices: (0..config.node_atlas_size).collect(),
            activated_nodes: default(),
        }
    }

    pub(crate) fn activate_node(&mut self, node: &mut NodeData) {
        node.atlas_index = self
            .available_indices
            .pop_front()
            .expect("Out of atlas ids.");

        self.activated_nodes.push((node.atlas_index, node.clone()));
    }

    pub(crate) fn deactivate_node(&mut self, node: &mut NodeData) {
        self.available_indices.push_front(node.atlas_index);

        node.atlas_index = Self::INACTIVE_ID;
    }
}
