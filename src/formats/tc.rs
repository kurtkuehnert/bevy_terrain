use crate::preprocess::file_io::{format_directory, iterate_directory};
use crate::terrain_data::NodeId;
use anyhow::Result;
use bevy::utils::HashSet;
use bincode::{config, Decode, Encode};
use std::{fs, path::Path};

#[derive(Encode, Decode, Debug)]
pub struct TC {
    pub nodes: Vec<NodeId>,
}

impl TC {
    pub fn decode_alloc(encoded: &[u8]) -> Result<Self> {
        let config = config::standard();
        let decoded = bincode::decode_from_slice(encoded, config)?;
        Ok(decoded.0)
    }

    pub fn encode_alloc(&self) -> Result<Vec<u8>> {
        let config = config::standard();
        let encoded = bincode::encode_to_vec(self, config)?;
        Ok(encoded)
    }

    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let encoded = fs::read(path)?;
        Self::decode_alloc(&encoded)
    }

    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let encoded = self.encode_alloc()?;
        fs::write(path, encoded)?;
        Ok(())
    }
}

/// Saves the node configuration of the terrain, which stores the [`NodeId`]s of all the nodes
/// of the terrain.
pub(crate) fn save_node_config(path: &str) {
    let mut tc = TC { nodes: vec![] };
    let attachment_directory = format_directory(path, "height");

    for (name, _) in iterate_directory(&attachment_directory) {
        let node_id = name.parse::<NodeId>().unwrap();
        tc.nodes.push(node_id);
    }

    tc.save_file(format_directory(path, "../config.tc"))
        .unwrap();
}

/// Loads the node configuration of the terrain, which stores the [`NodeId`]s of all the nodes
/// of the terrain.
pub(crate) fn load_node_config(path: &str) -> HashSet<NodeId> {
    let tc = TC::load_file(format_directory(path, "../config.tc")).unwrap();
    tc.nodes.into_iter().collect()
}
