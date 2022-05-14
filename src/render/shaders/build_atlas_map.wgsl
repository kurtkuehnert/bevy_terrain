#import bevy_terrain::config
#import bevy_terrain::node
#import bevy_terrain::parameters

[[group(0), binding(0)]]
var<uniform> config: TerrainConfig;
[[group(0), binding(1)]]
var quadtree: texture_2d<u32>;
[[group(0), binding(2)]]
var<storage, read_write> parameters: Parameters;
[[group(0), binding(3)]]
var<storage> node_list: NodeList;
[[group(0), binding(4)]]
var atlas_map: texture_storage_2d<rgba8uint, write>;

[[stage(compute), workgroup_size(1, 1, 1)]]
fn build_atlas_map(
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

    textureStore(atlas_map, chunk_position, vec4<u32>(node_position.lod, atlas_index / 256u, atlas_index % 256u, 0u));
}