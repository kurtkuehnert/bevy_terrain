use crate::{
    preprocess::file_io::{format_directory, iterate_directory},
    terrain_data::NodeCoordinate,
};
use anyhow::Result;
use bevy::utils::HashSet;
use bincode::{config, Decode, Encode};
use std::{fs, path::Path};

#[derive(Encode, Decode, Debug)]
pub struct TC {
    pub nodes: Vec<NodeCoordinate>,
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

/// Saves the node configuration of the terrain, which stores the [`NodeCoordinate`]s of all the nodes
/// of the terrain.
pub(crate) fn save_node_config(path: &str) {
    let mut tc = TC { nodes: vec![] };
    let attachment_directory = format_directory(path, "height");

    for (name, _) in iterate_directory(&attachment_directory) {
        let node_coordinate = name.parse::<NodeCoordinate>().unwrap();
        tc.nodes.push(node_coordinate);
    }

    tc.save_file(format_directory(path, "../config.tc"))
        .unwrap();
}

/// Loads the node configuration of the terrain, which stores the [`NodeCoordinate`]s of all the nodes
/// of the terrain.
pub(crate) fn load_node_config(path: &str) -> HashSet<NodeCoordinate> {
    if let Ok(tc) = TC::load_file(format_directory(path, "../config.tc")) {
        tc.nodes.into_iter().collect()
    } else {
        HashSet::default()
    }
}
