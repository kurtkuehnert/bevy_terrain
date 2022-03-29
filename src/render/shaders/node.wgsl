#define_import_path bevy_terrain::node

let INACTIVE_ID: u32 = 65534u;

struct NodePosition {
    lod: u32;
    x: u32;
    y: u32;
};

fn node_id(lod: u32, x: u32, y: u32) -> u32 {
    return (lod & 0xFu) << 28u | (x & 0x3FFFu) << 14u | (y & 0x3FFFu);
}

fn node_position(id: u32) -> NodePosition {
    return NodePosition((id >> 28u) & 0xFu, (id >> 14u) & 0x3FFFu, id & 0x3FFFu);
}

struct NodeList {
    data: array<u32>;
};
