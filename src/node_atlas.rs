use crate::quadtree::NodeData;
use crate::quadtree_update::NodeUpdate;
use bevy::prelude::*;

/// Maps the assets to the corresponding active nodes and tracks the node updates.
#[derive(Component)]
pub struct NodeAtlas {
    pub(crate) height_maps: Vec<Handle<Image>>,
    pub(crate) available_ids: Vec<u16>,
}

impl NodeAtlas {
    pub(crate) const NONEXISTING_ID: u16 = u16::MAX;
    pub(crate) const INACTIVE_ID: u16 = u16::MAX - 1;

    pub fn new(atlas_size: u16) -> Self {
        Self {
            height_maps: vec![Handle::default(); atlas_size as usize],
            available_ids: (0..atlas_size).collect(),
        }
    }

    pub(crate) fn add_node(&mut self, node: &mut NodeData, updates: &mut Vec<NodeUpdate>) {
        let atlas_index = self.available_ids.pop().expect("Out of atlas ids.");

        self.height_maps[atlas_index as usize] = node.height_map.as_weak();
        node.atlas_index = atlas_index;

        updates.push(NodeUpdate {
            node_id: node.id,
            atlas_index: node.atlas_index as u32,
        });
    }

    pub(crate) fn remove_node(&mut self, node: &mut NodeData, updates: &mut Vec<NodeUpdate>) {
        self.available_ids.push(node.atlas_index);

        node.atlas_index = Self::INACTIVE_ID;

        updates.push(NodeUpdate {
            node_id: node.id,
            atlas_index: node.atlas_index as u32,
        });
    }
}
