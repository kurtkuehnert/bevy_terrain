use crate::formats::tc::TC;
use crate::preprocess::file_io::{format_directory, iterate_directory};
use crate::terrain_data::NodeId;
use crate::TerrainConfig;

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

pub fn load_node_config(config: &mut TerrainConfig) {
    let tc = TC::load_file(format_directory(&config.path, "../config.tc")).unwrap();
    config.nodes = tc.nodes.into_iter().collect();
}
