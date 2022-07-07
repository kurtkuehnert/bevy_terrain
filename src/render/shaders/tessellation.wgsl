#import bevy_terrain::config
#import bevy_terrain::parameters
#import bevy_terrain::patch

// Todo: increase workgroup size

struct CullData {
    world_position: vec4<f32>;
    view_proj: mat4x4<f32>;
    model: mat4x4<f32>;
    planes: array<vec4<f32>, 6>;
};

[[group(0), binding(0)]]
var<uniform> config: TerrainViewConfig;
[[group(0), binding(1)]]
var<storage, read_write> parameters: Parameters;
[[group(0), binding(2)]]
var<storage, read_write> temporary_patches: PatchList;
[[group(0), binding(3)]]
var<storage, read_write> final_patches: PatchList;

[[group(1), binding(0)]]
 var<uniform> cull_data: CullData;

//  MIT License. © Ian McEwan, Stefan Gustavson, Munrocket
//
 fn permute3(x: vec3<f32>) -> vec3<f32> { return (((x * 34.) + 1.) * x) % vec3<f32>(289.); }

 fn simplexNoise2(v: vec2<f32>) -> f32 {
   let C = vec4<f32>(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
   var i: vec2<f32> = floor(v + dot(v, C.yy));
   let x0 = v - i + dot(i, C.xx);
   var i1: vec2<f32> = select(vec2<f32>(1., 0.), vec2<f32>(0., 1.), (x0.x > x0.y));
   var x12: vec4<f32> = x0.xyxy + C.xxzz - vec4<f32>(i1, 0., 0.);
   i = i % vec2<f32>(289.);
   let p = permute3(permute3(i.y + vec3<f32>(0., i1.y, 1.)) + i.x + vec3<f32>(0., i1.x, 1.));
   var m: vec3<f32> = max(0.5 -
       vec3<f32>(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3<f32>(0.));
   m = m * m;
   m = m * m;
   let x = 2. * fract(p * C.www) - 1.;
   let h = abs(x) - 0.5;
   let ox = floor(x + 0.5);
   let a0 = x - ox;
   m = m * (1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h));
   let g = vec3<f32>(a0.x * x0.x + h.x * x0.y, a0.yz * x12.xz + h.yz * x12.yw);
   return 130. * dot(m, g);
 }

fn divide(patch_x: u32, patch_y: u32, size: u32) -> bool {
    var divide = false;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let x = f32(patch_x + (i       & 1u));
        let y = f32(patch_y + (i >> 1u & 1u));

        let local_position = vec2<f32>(x, y) * config.patch_scale * f32(size);
        let world_position = vec3<f32>(local_position.x, config.height_under_viewer, local_position.y);
        let distance = length(cull_data.world_position.xyz - world_position) * 0.99; // consider adding a small error mitigation

        divide = divide || (distance < f32(size >> 1u) * config.view_distance);
    }

    return divide;
}

fn frustum_cull(position: vec2<f32>, size: f32) -> bool {
    let aabb_min = vec3<f32>(position.x, 0.0, position.y);
    let aabb_max = vec3<f32>(position.x + size, 1000.0, position.y + size);

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

    for (var i = 0; i < 5; i = i + 1) {
        let plane = cull_data.planes[i];

        var in = 0u;

        for (var j = 0; j < 8; j = j + 1) {
            let corner = corners[j];

            if (dot(plane, corner) < 0.0) {
                in = in + 1u;
            }

            if (in == 0u) {
                return true;
            }
        }
    }

    return false;
}

fn child_index() -> i32 {
    return atomicAdd(&parameters.child_index, parameters.counter);
}

fn final_index(lod: u32) -> i32 {
    if (lod == 0u) {
        return atomicAdd(&parameters.final_index1, 1);
    }
    if (lod == 1u) {
        return atomicAdd(&parameters.final_index2, 1) + 100000;
    }
    if (lod == 2u) {
        return atomicAdd(&parameters.final_index3, 1) + 200000;
    }
    if (lod == 3u) {
        return atomicAdd(&parameters.final_index4, 1) + 300000;
    }
    return 0;
    // return atomicAdd(&parameters.final_indices[i32(lod)], 1) + i32(lod) * 1000000;
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

    temporary_patches.data[child_index()] = Patch(vec2<u32>(x, y), size, stitch, 1u, 0u);
}

fn patch_lod(coords: vec2<u32>, size: u32) -> i32 {
    let local_position = (vec2<f32>(coords) + 0.5) * config.patch_scale * f32(size);
    return i32((simplexNoise2(local_position / 1600.0) + 1.0) / 2.0 * 4.0);
}

fn add_final_patch(patch: Patch) {
    var stitch = 0u;
    var morph = 0u;
    var directions = array<vec2<i32>, 4>(
        vec2<i32>(-1,  0),
        vec2<i32>( 0, -1),
        vec2<i32>( 1,  0),
        vec2<i32>( 0,  1)
    );

#ifdef DENSITY
    let coords = vec2<i32>(patch.coords);
    let lod = patch_lod(patch.coords, patch.size);
    let parent_lod = patch_lod(patch.coords >> vec2<u32>(1u), patch.size << 1u);

    for (var i = 0; i < 4; i = i + 1) {
        let neighbour_coords = vec2<u32>(coords + directions[i]);
        let neighbour_lod = patch_lod(neighbour_coords, patch.size);
        let neighbour_parent_lod = patch_lod(neighbour_coords >> vec2<u32>(1u), patch.size << 1u);

        // stitch = stitch | (lod_diff   << u32(i * 8));
        stitch = stitch |  calc_patch_size(u32(min(neighbour_lod,        lod)))               << u32(i * 8);
        morph  = morph  | (calc_patch_size(u32(min(neighbour_parent_lod, parent_lod))) >> 1u) << u32(i * 8);
    }
#endif
#ifndef DENSITY
    let lod = 2u;
    let parent_lod = lod;

    for (var i = 0; i < 4; i = i + 1) {
        stitch = stitch |  calc_patch_size(lod)               << u32(i * 8);
        morph  = morph  | (calc_patch_size(parent_lod) >> 1u) << u32(i * 8);
    }
#endif

    // Todo: fix stitching patches of different lod levels

    var patch = patch;
    patch.stitch = stitch;
    patch.morph = morph;
    patch.parent_count = calc_patch_size(u32(parent_lod)) >> 1u;
    final_patches.data[final_index(u32(lod))] = patch;
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn refine_patches(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>,
) {
    var parent_patch = temporary_patches.data[parent_index(invocation_id.x)];
    let parent_coords = parent_patch.coords;

    if (divide(parent_coords.x, parent_coords.y, parent_patch.size)) {
        let size = parent_patch.size >> 1u;

        //       bit |      3 |      2 |      1 |      0
        // direction | bottom |  right |    top |   left
        var stitch = (u32(!divide(parent_coords.x - 1u, parent_coords.y, parent_patch.size)) << 1u)
                   | (u32(!divide(parent_coords.x, parent_coords.y - 1u, parent_patch.size)) << 1u) << 8u
                   | (u32(!divide(parent_coords.x + 1u, parent_coords.y, parent_patch.size)) << 1u) << 16u
                   | (u32(!divide(parent_coords.x, parent_coords.y + 1u, parent_patch.size)) << 1u) << 24u;

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
            let local_position = vec2<f32>(f32(x), f32(y)) * config.patch_scale * f32(size);
            if (local_position.x > f32(config.terrain_size) || local_position.y > f32(config.terrain_size)) {
                continue;
            }

            // if (frustum_cull(local_position, config.patch_scale * f32(config.patch_size * size))) {
            //     continue;
            // }

            temporary_patches.data[child_index()] = Patch(vec2<u32>(x, y), size, stitch, 1u, 0u);
        }
    }
    else {
        add_final_patch(parent_patch);
    }
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn select_finest_patches(
    [[builtin(global_invocation_id)]] invocation_id: vec3<u32>,
) {
    add_final_patch(temporary_patches.data[parent_index(invocation_id.x)]);
}


