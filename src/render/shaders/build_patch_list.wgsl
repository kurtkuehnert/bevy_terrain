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
};

struct Patch {
    position: vec2<u32>;
    scale: u32;
    atlas_index: u32;
    coord_offset: u32;
    lod: u32;
};

struct PatchList {
    data: array<Patch>;
};

struct CullData {
    view_proj: mat4x4<f32>;
    model: mat4x4<f32>;
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
var<storage, read_write> patch_list: PatchList;
[[group(1), binding(0)]]
var<uniform> cull_data: CullData;

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
    patch.scale = 1u << node_position.lod;
    let patch_size = patch.scale * config.patch_size;
    patch.position = (8u * vec2<u32>(node_position.x, node_position.y) + patch_id) * patch_size;
    patch.atlas_index = textureLoad(quadtree, vec2<i32>(i32(node_position.x), i32(node_position.y)), i32(node_position.lod)).x;
    patch.coord_offset = 8u * patch_id.y + patch_id.x;
    patch.lod = node_position.lod;

    // frustum culling

    let model_view_proj = cull_data.view_proj * cull_data.model;

    // getting the min and max y height correct is crucial or there will be boundig boxes,
    // where all points are outside the frustum (overfitting)
    let aabb_min = vec3<f32>(f32(patch.position.x), 0.0, f32(patch.position.y));
    let aabb_max = vec3<f32>(f32(patch.position.x + patch_size), 2000.0,
                             f32(patch.position.y + patch_size));


    var corners = array<vec4<f32>, 8>(
        vec4<f32>(aabb_min.x, aabb_min.y, aabb_min.z, 1.0),
        vec4<f32>(aabb_min.x, aabb_min.y, aabb_max.z, 1.0),
        vec4<f32>(aabb_min.x, aabb_max.y, aabb_min.z, 1.0),
        vec4<f32>(aabb_min.x, aabb_max.y, aabb_max.z, 1.0),
        vec4<f32>(aabb_max.x, aabb_min.y, aabb_min.z, 1.0),
        vec4<f32>(aabb_max.x, aabb_min.y, aabb_max.z, 1.0),
        vec4<f32>(aabb_max.x, aabb_max.y, aabb_min.z, 1.0),
        vec4<f32>(aabb_max.x, aabb_max.y, aabb_max.z, 1.0)
    );

    var inside = false;

    for (var i = 0; i < 8; i = i + 1) {
        let corner = model_view_proj * corners[i];

        inside = inside ||
            (-corner.w <= corner.x && corner.x <= corner.w &&
             -corner.w <= corner.y && corner.y <= corner.w &&
                   0.0 <= corner.z && corner.z <= corner.w);
    }

    if (inside) {
        let patch_index = atomicAdd(&parameters.patch_index, 1u);
        patch_list.data[patch_index] = patch;
    }
}
