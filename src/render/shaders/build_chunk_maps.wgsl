let INACTIVE_ID: u32 = 65534u;

struct TerrainConfig {
    lod_count: u32;
    chunk_size: u32;
    patch_size: u32;
    vertices_per_row: u32;
    area_count: vec2<u32>;
    scale: f32;
    height: f32;
};

struct NodePosition {
    lod: u32;
    x: u32;
    y: u32;
};

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
var<uniform> config: TerrainConfig;
[[group(0), binding(1)]]
var quadtree: texture_2d<u32>;
[[group(0), binding(2)]]
var<storage, read_write> parameters: Parameters;
[[group(0), binding(3)]]
var<storage> node_list: NodeList;
[[group(0), binding(4)]]
var lod_map: texture_storage_2d<r8uint, write>;
[[group(0), binding(5)]]
var atlas_map: texture_storage_2d<r16uint, write>;

[[stage(compute), workgroup_size(1, 1, 1)]]
fn build_chunk_maps(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>
) {
    let chunk_index = invocation_id.x;

    var i = 0u;
    var chunk_count = 0u;
    var node_count = 0u;
    var node_size = 0u;
    var node_scale = 0u;

    for (; i < config.lod_count; i = i + 1u) {
        node_scale = (1u << i);
        node_size = node_scale * node_scale;

        let next_node_count = parameters.node_counts[i];
        let next_chunk_count = next_node_count * node_size;

        if (chunk_index < chunk_count + next_chunk_count) {
            break;
        }

        node_count = node_count + next_node_count;
        chunk_count = chunk_count + next_chunk_count;
    }

    let chunk_offset = chunk_index - chunk_count;
    let node_index = parameters.final_index - 1u - (node_count + chunk_offset / node_size);
    let node_id = node_list.data[node_index];
    let node_position = node_position(node_id);

    let node_offset = chunk_offset % node_size;
    let chunk_position = vec2<i32>(
        i32(node_position.x * node_scale + node_offset / node_scale),
        i32(node_position.y * node_scale + node_offset % node_scale));

    let atlas_index = textureLoad(quadtree, vec2<i32>(i32(node_position.x), i32(node_position.y)), i32(node_position.lod)).x;

    textureStore(lod_map, chunk_position, vec4<u32>(node_position.lod));
    textureStore(atlas_map, chunk_position, vec4<u32>(atlas_index));
}