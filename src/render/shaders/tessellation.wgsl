#import bevy_terrain::config
#import bevy_terrain::parameters
#import bevy_terrain::patch

struct CullData {
    world_position: vec4<f32>;
    view_proj: mat4x4<f32>;
    model: mat4x4<f32>;
};

[[group(0), binding(0)]]
var<uniform> config: TerrainConfig;
[[group(0), binding(1)]]
var<storage, read_write> parameters: Parameters;
[[group(0), binding(2)]]
var<storage, read_write> parent_list: PatchList;
[[group(0), binding(3)]]
var<storage, read_write> child_list: PatchList;
[[group(0), binding(4)]]
var<storage, read_write> final_list: PatchList;

[[group(1), binding(0)]]
var<uniform> cull_data: CullData;

fn divide(x: u32, y: u32, size: u32) -> bool {
    let world_position = vec2<f32>(f32(x + 4u), f32(y + 4u)) * f32(size);
    let distance = length(cull_data.world_position.xz - world_position);

    return distance < f32(size) * config.view_distance;
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn select_coarsest_patches(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>,
) {
    let x = invocation_id.x * config.patch_size;
    let y = invocation_id.y * config.patch_size;
    let size = 1u << (config.lod_count - 1u);
    let stitch = 0u; // no stitch required

    let child_index = atomicAdd(&parameters.child_index, 1u);
    child_list.data[child_index] = Patch(x, y, size, stitch);
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn refine_patches(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>,
) {
    let parent_index = invocation_id.x;
    let parent_patch = parent_list.data[parent_index];

    if (divide(parent_patch.x, parent_patch.y, parent_patch.size)) {
        let size = parent_patch.size >> 1u;

        //       bit |      3 |      2 |      1 |      0
        // direction | bottom |  right |    top |   left
        var stitch = u32(!divide(parent_patch.x - config.patch_size, parent_patch.y, parent_patch.size))
                   | u32(!divide(parent_patch.x, parent_patch.y - config.patch_size, parent_patch.size)) << 1u
                   | u32(!divide(parent_patch.x + config.patch_size, parent_patch.y, parent_patch.size)) << 2u
                   | u32(!divide(parent_patch.x, parent_patch.y + config.patch_size, parent_patch.size)) << 3u;

        //    i |    3 |    2 |    1 |    0
        //  x y |  1 1 |  0 1 |  1 0 |  0 0
        // mask | 1100 | 1001 | 0110 | 0011
        // mask |    C |    9 |    6 |    3
        for (var i: u32 = 0u; i < 4u; i = i + 1u) {
            let x = (parent_patch.x << 1u) + (i       & 1u) * config.patch_size;
            let y = (parent_patch.y << 1u) + (i >> 1u & 1u) * config.patch_size;

            // select two adjacent edges on parent level
            let stitch = stitch & (0xC963u >> (i << 2u));

            let child_index = atomicAdd(&parameters.child_index, 1u);
            child_list.data[child_index] = Patch(x, y, size, stitch);
        }
    }
    else {
        let final_index = atomicAdd(&parameters.final_index, 1u);
        final_list.data[final_index] = parent_patch;
    }
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn select_finest_patches(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>,
) {
    let parent_index = invocation_id.x;

    let final_index = atomicAdd(&parameters.final_index, 1u);
    final_list.data[final_index] = parent_list.data[parent_index];
}


