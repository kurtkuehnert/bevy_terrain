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
    lod_delta: u32; // should be u16
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
[[group(0), binding(5)]]
var lod_map: texture_2d<u32>;
[[group(1), binding(0)]]
var<uniform> cull_data: CullData;

fn calculate_lod_transition(patch_id: vec2<u32>, node_position: NodePosition, scale: u32) -> u32 {
    let position = vec2<i32>(
        i32(node_position.x * scale + patch_id.x * (scale >> 3u)),
        i32(node_position.y * scale + patch_id.y * (scale >> 3u))
    );

    let lod = i32(node_position.lod);
    var lod_delta = 0u;

    if (patch_id.x == 0u) {
        let left_lod = i32(textureLoad(lod_map, position + vec2<i32>(-1, 0), 0).x);
        lod_delta = lod_delta | (u32(max(left_lod - lod, 0)) << 12u);
    }
    if (patch_id.y == 0u) {
        let top_lod = i32(textureLoad(lod_map, position + vec2<i32>(0, -1), 0).x);
        lod_delta = lod_delta | (u32(max(top_lod - lod, 0)) << 8u);
    }
    if (patch_id.x == 7u) {
        let right_lod = i32(textureLoad(lod_map, position + vec2<i32>(i32(scale), 0), 0).x);
        lod_delta = lod_delta | (u32(max(right_lod - lod, 0)) << 4u);
    }
    if (patch_id.y == 7u) {
        let bottom_lod = i32(textureLoad(lod_map, position + vec2<i32>(0, i32(scale)), 0).x);
        lod_delta = lod_delta | u32(max(bottom_lod - lod, 0));
    }

    return lod_delta;
}

fn frustum_cull(position: vec2<f32>, size: f32) -> bool {
    let model_view_proj = cull_data.view_proj * cull_data.model;

    // getting the min and max y height correct is crucial or there will be boundig boxes,
    // where all points are outside the frustum (overfitting)
    let aabb_min = vec3<f32>(position.x, 0.0, position.y);
    let aabb_max = vec3<f32>(position.x + size, 2000.0, position.y + size);

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

    var visible = false;

    for (var i = 0; i < 8; i = i + 1) {
        let corner = model_view_proj * corners[i];

        visible = visible ||
            (-corner.w <= corner.x && corner.x <= corner.w &&
             -corner.w <= corner.y && corner.y <= corner.w &&
                   0.0 <= corner.z && corner.z <= corner.w);
    }

    return visible;
}

[[stage(compute), workgroup_size(8, 8, 1)]]
fn build_patch_list(
    [[builtin(workgroup_id)]] workgroup_id: vec3<u32>,
    [[builtin(local_invocation_id)]] local_id: vec3<u32>
) {
    let patch_id = local_id.xy;
    let node_index = workgroup_id.x;
    let node_id = node_list.data[node_index];
    let node_position = node_position(node_id);

    let scale = 1u << node_position.lod;
    let patch_size = scale * config.patch_size;

    var patch: Patch;
    patch.position = patch_size * (vec2<u32>(node_position.x << 3u, node_position.y << 3u) + patch_id);
    patch.scale = scale;
    patch.atlas_index = textureLoad(quadtree, vec2<i32>(i32(node_position.x), i32(node_position.y)), i32(node_position.lod)).x;
    patch.coord_offset = patch_id.x | (patch_id.y << 3u);
    patch.lod = node_position.lod;
    patch.lod_delta = calculate_lod_transition(patch_id, node_position, scale);

    let visible = frustum_cull(vec2<f32>(patch.position), f32(patch_size));
    let visible = true;

    if (visible) {
        let patch_index = atomicAdd(&parameters.patch_index, 1u);
        patch_list.data[patch_index] = patch;
    }
}
