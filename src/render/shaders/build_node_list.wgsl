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

struct Parameters {
    child_index: atomic<u32>;
    final_index: atomic<u32>;
    patch_index: atomic<u32>;
    lod: u32;
    previous_node_count: u32;
    node_counts: array<u32, 16>;
};

[[group(0), binding(0)]]
var quadtree: texture_2d<u32>;
[[group(0), binding(1)]]
var<storage, read_write> parameters: Parameters;
[[group(0), binding(2)]]
var<storage, read_write> parent_list: NodeList;
[[group(0), binding(3)]]
var<storage, read_write> child_list: NodeList;
[[group(0), binding(4)]]
var<storage, read_write> final_list: NodeList;

[[stage(compute), workgroup_size(1, 1, 1)]]
fn build_area_list(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>
) {
    let x = invocation_id.x;
    let y = invocation_id.y;
    let lod = parameters.lod - 1u;
    let id = node_id(lod, x, y);
    let atlas_id = textureLoad(quadtree, vec2<i32>(i32(x), i32(y)), i32(lod)).x;

    // assume that area nodes are allways loaded
    //
    // if (atlas_id < INACTIVE_ID) {
    //     let child_index = atomicAdd(&parameters.child_index, 1u);
    //     child_list.data[child_index] = id;
    // }

    let child_index = atomicAdd(&parameters.child_index, 1u);
    child_list.data[child_index] = id;
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn build_node_list(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>
) {
    let parent_index = invocation_id.x;
    let parent_id = parent_list.data[parent_index];
    let parent_position = node_position(parent_id);

    var loaded: u32 = 0u;
    var child_ids: array<u32, 4>;
    let lod = parent_position.lod - 1u;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let x = (parent_position.x << 1u) + (i & 1u);
        let y = (parent_position.y << 1u) + ((i >> 1u) & 1u);
        child_ids[i] = node_id(lod, x, y);
        let atlas_id = textureLoad(quadtree, vec2<i32>(i32(x), i32(y)), i32(lod)).x;

        if (atlas_id < INACTIVE_ID) {
            loaded = loaded + (1u << i);
        }
    }

    if (loaded == 0xFu) {
        for (var i: u32 = 0u; i < 4u; i = i + 1u) {
            let child_index = atomicAdd(&parameters.child_index, 1u);
            child_list.data[child_index] = child_ids[i];
        }
    }
    else {
        let final_index = atomicAdd(&parameters.final_index, 1u);
        final_list.data[final_index] = parent_id;
    }
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn build_chunk_list(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>
) {
    let parent_index = invocation_id.x;

    let final_index = atomicAdd(&parameters.final_index, 1u);
    final_list.data[final_index] = parent_list.data[parent_index];
}


