#import bevy_terrain::config
#import bevy_terrain::parameters
#import bevy_terrain::patch

// Todo: increase workgroup size

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
var<storage, read_write> temporary_list: PatchList;
[[group(0), binding(3)]]
var<storage, read_write> final_list: PatchList;

[[group(1), binding(0)]]
 var<uniform> cull_data: CullData;

fn divide(patch_x: u32, patch_y: u32, size: u32) -> bool {
    var divide = false;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let x = f32(patch_x + (i       & 1u));
        let y = f32(patch_y + (i >> 1u & 1u));

        let local_position = vec2<f32>(x, y) * config.patch_scale * f32(config.patch_size * size);
        let world_position = vec3<f32>(local_position.x, config.height / 2.0, local_position.y);
        let distance = length(cull_data.world_position.xyz - world_position);

        divide = divide || (distance < f32(size >> 1u) * config.view_distance);
    }

    return divide;
}

fn child_index() -> i32 {
    return atomicAdd(&parameters.child_index, parameters.counter);
}

fn final_index() -> i32 {
    return atomicAdd(&parameters.final_index, 1);
}

fn parent_index(id: u32) -> i32 {
    return i32(config.patch_count - 1u) * clamp(parameters.counter, 0, 1) - i32(id) * parameters.counter;
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn select_coarsest_patches(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>,
) {
    let x = invocation_id.x;
    let y = invocation_id.y;
    let size = 1u << config.refinement_count;
    let stitch = 0u; // no stitch required

    temporary_list.data[child_index()] = Patch(vec2<u32>(x, y), size, stitch);
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn refine_patches(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>,
) {
    let parent_patch = temporary_list.data[parent_index(invocation_id.x)];
    let parent_coords = parent_patch.coords;

    if (divide(parent_coords.x, parent_coords.y, parent_patch.size)) {
        let size = parent_patch.size >> 1u;

        //       bit |      3 |      2 |      1 |      0
        // direction | bottom |  right |    top |   left
        var stitch = u32(!divide(parent_coords.x - 1u, parent_coords.y, parent_patch.size))
                   | u32(!divide(parent_coords.x, parent_coords.y - 1u, parent_patch.size)) << 1u
                   | u32(!divide(parent_coords.x + 1u, parent_coords.y, parent_patch.size)) << 2u
                   | u32(!divide(parent_coords.x, parent_coords.y + 1u, parent_patch.size)) << 3u;

        //    i |    3 |    2 |    1 |    0
        //  x y |  1 1 |  0 1 |  1 0 |  0 0
        // mask | 1100 | 1001 | 0110 | 0011
        // mask |    C |    9 |    6 |    3
        for (var i: u32 = 0u; i < 4u; i = i + 1u) {
            let x = (parent_coords.x << 1u) + (i       & 1u);
            let y = (parent_coords.y << 1u) + (i >> 1u & 1u);

            // select two adjacent edges on parent level
            let stitch = stitch & (0xC963u >> (i << 2u));

            // cull patches outside of the terrain
            let local_position = vec3<f32>(f32(x), 0.0, f32(y)) * config.patch_scale * f32(config.patch_size * size);
            if (local_position.x > f32(config.terrain_size) || local_position.z > f32(config.terrain_size)) {
                continue;
            }

            temporary_list.data[child_index()] = Patch(vec2<u32>(x, y), size, stitch);
        }
    }
    else {
        final_list.data[final_index()] = parent_patch;
    }
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn select_finest_patches(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>,
) {
    final_list.data[final_index()] = temporary_list.data[parent_index(invocation_id.x)];
}


