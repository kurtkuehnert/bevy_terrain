struct TerrainConfig {
    lod_count: u32;
    chunk_size: u32;
    patch_size: u32;
    index_count: u32;
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

struct Patch {
    position: vec2<u32>;
    size: u32;
    atlas_index: u32;
    coord_offset: u32;
    lod: u32;
};

struct PatchList {
    data: array<Patch>;
};

[[group(0), binding(0)]]
var<uniform> config: TerrainConfig;
[[group(0), binding(1)]]
var quadtree: texture_2d<u32>;
[[group(0), binding(2)]]
var<storage> node_list: NodeList;
[[group(0), binding(3)]]
var<storage, read_write> patch_list: PatchList;

[[stage(compute), workgroup_size(8, 8, 1)]]
fn build_patch_list(
    [[builtin(workgroup_id)]] workgroup_id: vec3<u32>,
    [[builtin(local_invocation_id)]] local_id: vec3<u32>
) {
    let patch_id = local_id.xy;
    let node_index = workgroup_id.x;
    let node_id = node_list.data[node_index];
    let node_position = node_position(node_id);

    var patch: Patch;
    patch.size = config.patch_size * (1u << node_position.lod);
    patch.position = (8u * vec2<u32>(node_position.x, node_position.y) + patch_id) * patch.size;
    patch.atlas_index = textureLoad(quadtree, vec2<i32>(i32(node_position.x), i32(node_position.y)), i32(node_position.lod)).x;
    patch.coord_offset = 8u * patch_id.y + patch_id.x;
    patch.lod = node_position.lod;

    let patch_index = 64u * node_index + patch.coord_offset;

    patch_list.data[patch_index] = patch;
}
