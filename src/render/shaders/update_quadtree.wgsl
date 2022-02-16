struct NodePosition {
    lod: u32;
    x: u32;
    y: u32;
};

fn node_position(id: u32) -> NodePosition {
    return NodePosition((id >> 28u) & 0xFu, (id >> 14u) & 0x3FFFu, id & 0x3FFFu);
}

struct NodeUpdate {
    node_id: u32;
    atlas_index: u32;
};

struct QuadtreeUpdate {
    data: array<NodeUpdate>;
};

[[group(0), binding(0)]]
var quadtree_layer: texture_storage_2d<r16uint, write>;
[[group(0), binding(1)]]
var<storage> quadtree_update: QuadtreeUpdate;

[[stage(compute), workgroup_size(1, 1, 1)]]
fn update_quadtree(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>
) {
    let index = invocation_id.x;
    let update = quadtree_update.data[index];
    let position = node_position(update.node_id);

    textureStore(quadtree_layer, vec2<i32>(i32(position.x), i32(position.y)), vec4<u32>(update.atlas_index));
}
