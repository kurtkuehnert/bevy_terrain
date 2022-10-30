use crate::{
    formats::tc::TC,
    preprocess::file_io::{format_directory, iterate_directory},
    terrain_data::NodeId,
    TerrainConfig,
};

/// Saves. s the node configuration of the terrain, which stores the [`NodeId`]s of all the nodes
/// of the terrain.
pub fn save_config(config: &TerrainConfig) {
    let mut tc = TC { nodes: vec![] };
    let attachment_directory = format_directory(&config.path, &config.attachments[0].name);

    for (name, _) in iterate_directory(&attachment_directory) {
        let node_id = name.parse::<NodeId>().unwrap();
        tc.nodes.push(node_id);
    }

    tc.save_file(format_directory(&config.path, "../config.tc"))
        .unwrap();
}

/// Loads the node configuration of the terrain, which stores the [`NodeId`]s of all the nodes
/// of the terrain.
pub fn load_node_config(config: &mut TerrainConfig) {
    let tc = TC::load_file(format_directory(&config.path, "../config.tc")).unwrap();
    config.nodes = tc.nodes.into_iter().collect();
}
