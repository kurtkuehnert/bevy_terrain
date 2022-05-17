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

struct NodeActivation {
    node_id: u32; // node to update
    atlas_index: u32; // new atlas index
    lod: u32; // lod of atlas entry
};

struct NodeDeactivation {
    node_id: u32; // node to update
    ancestor_id: u32; // active ancestor node
};

struct NodeActivations {
    data: array<NodeActivation>;
};

struct NodeDeactivations {
    data: array<NodeDeactivation>;
};

[[group(0), binding(0)]]
var quadtree: texture_storage_2d_array<rgba8uint, read_write>;
[[group(0), binding(1)]]
var<storage> node_activations: NodeActivations;
[[group(0), binding(2)]]
var<storage> node_deactivations: NodeDeactivations;


// Todo: consider increasing the workgroup size and pre sorting the updates for better cache coherenze
[[stage(compute), workgroup_size(1, 1, 1)]]
fn activate_nodes([[builtin(global_invocation_id)]] invocation_id: vec3<u32>) {
    let index = invocation_id.x;
    let update = node_activations.data[index];
    let position = node_position(update.node_id);

    let output = vec4<u32>(update.atlas_index >> 8u, update.atlas_index & 0xFFu, update.lod,  0u);
    textureStore(quadtree, vec2<i32>(i32(position.x), i32(position.y)), i32(position.lod), output);
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn deactivate_nodes([[builtin(global_invocation_id)]] invocation_id: vec3<u32>) {
    let index = invocation_id.x;
    let update = node_deactivations.data[index];
    let position = node_position(update.node_id);
    let ancestor_position = node_position(update.ancestor_id);

    let quadtree_entry = textureLoad(quadtree, vec2<i32>(i32(ancestor_position.x), i32(ancestor_position.y)), i32(ancestor_position.lod));
    textureStore(quadtree, vec2<i32>(i32(position.x), i32(position.y)), i32(position.lod), quadtree_entry);
}

