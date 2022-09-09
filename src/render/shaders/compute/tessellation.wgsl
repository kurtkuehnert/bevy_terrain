#import bevy_terrain::types
#import bevy_terrain::parameters

// Todo: increase workgroup size

struct TerrainConfig {
    lod_count: u32,
    height: f32,
    chunk_size: u32,
    terrain_size: u32,
    height_scale: f32,
    minmax_scale: f32,
    _empty: u32,
    _empty: u32,
    height_offset: f32,
    minmax_offset: f32,
    _empty: u32,
    _empty: u32,
}

struct CullingData {
    world_position: vec4<f32>,
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    planes: array<vec4<f32>, 5>,
}

@group(0) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(0) @binding(1)
var quadtree: texture_2d_array<u32>;
@group(0) @binding(2)
var<storage, read_write> final_tiles: TileList;
@group(0) @binding(3)
var<storage, read_write> temporary_tiles: TemporaryTileList;
@group(0) @binding(4)
var<storage, read_write> parameters: Parameters;

@group(1) @binding(0)
var<uniform> view: CullingData;

 // terrain bindings
@group(2) @binding(0)
var<uniform> config: TerrainConfig;
@group(2) @binding(1)
var terrain_sampler: sampler;
@group(2) @binding(2)
var height_atlas: texture_2d_array<f32>;
@group(2) @binding(3)
var minmax_atlas: texture_2d_array<f32>;

#import bevy_terrain::atlas
#import bevy_terrain::functions

fn child_index() -> i32 {
    return atomicAdd(&parameters.child_index, parameters.counter);
}

fn parent_index(id: u32) -> i32 {
    return i32(view_config.tile_count - 1u) * clamp(parameters.counter, 0, 1) - i32(id) * parameters.counter;
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
    // return atomicAdd(&parameters.final_indices[lod], 1) + i32(lod) * 1000000;
}

fn frustum_cull(tile: TileInfo) -> bool {
    let size = f32(tile.size) * view_config.tile_scale;
    let local_position = (vec2<f32>(tile.coords) + 0.5) * size;

    let minmax = minmax(local_position, size);
    // let minmax = vec2<f32>(0.0, config.height); // Todo: fix this

    // frustum culling optimized
    let aabb_min = vec3<f32>(local_position.x - size / 2.0, minmax.x, local_position.y - size / 2.0);
    let aabb_max = vec3<f32>(local_position.x + size / 2.0, minmax.y, local_position.y + size / 2.0);

    for (var i = 0; i < 5; i = i + 1) {
        let plane = view.planes[i];

        var p_corner = vec4<f32>(aabb_min.x, aabb_min.y, aabb_min.z, 1.0);
        var n_corner = vec4<f32>(aabb_max.x, aabb_max.y, aabb_max.z, 1.0);
        if (plane.x >= 0.0) { p_corner.x = aabb_max.x; n_corner.x = aabb_min.x; }
        if (plane.y >= 0.0) { p_corner.y = aabb_max.y; n_corner.y = aabb_min.y; }
        if (plane.z >= 0.0) { p_corner.z = aabb_max.z; n_corner.z = aabb_min.z; }

    	if (dot(plane, p_corner) < 0.0) {
    	    // the clostest corner is outside the plane -> cull
    	    return true;
    	}
    	if (dot(plane, n_corner) < 0.0) {
    	    // the furthest corner is inside the plane -> don't cull
    	    return false;
    	}
    }

    return false;

    // frustum culling bevy
    // let center_position = vec4<f32>(local_position.x, (minmax.y + minmax.x) / 2.0, local_position.y, 1.0);
    // let half_extents    = vec3<f32>(size,              minmax.y - minmax.x,        size) / 2.0;
    //
    // // let size = f32(tile.size) * view_config.tile_scale;
    // // let local_position = (vec2<f32>(tile.coords) + 0.5) * size;
    // // let center_position = vec4<f32>(local_position.x, 500.0, local_position.y, 1.0);
    // // let half_extends = vec3<f32>(size / 2.0, 500.0, size / 2.0);
    //
    // for (var i = 0; i < 6; i = i + 1) {
    //     let p_normal_d = view.planes[i];
    //     let relative_radius = dot(abs(p_normal_d.xyz), half_extents);
    //
    //     if (dot(p_normal_d, center_position) + relative_radius <= 0.0) {
    //         // no intersection -> cull
    //         return true;
    //     }
    // }
    //
    // return false;

    // frustum culling naive
    // let aabb_min = vec3<f32>(local_position.x - size / 2.0, minmax.x, local_position.y - size / 2.0);
    // let aabb_max = vec3<f32>(local_position.x + size / 2.0, minmax.y, local_position.y + size / 2.0);
    //
    // var corners = array<vec4<f32>, 8>(
    //     vec4<f32>(aabb_min.x, aabb_min.y, aabb_min.z, 1.0),
    //     vec4<f32>(aabb_min.x, aabb_min.y, aabb_max.z, 1.0),
    //     vec4<f32>(aabb_min.x, aabb_max.y, aabb_min.z, 1.0),
    //     vec4<f32>(aabb_min.x, aabb_max.y, aabb_max.z, 1.0),
    //     vec4<f32>(aabb_max.x, aabb_min.y, aabb_min.z, 1.0),
    //     vec4<f32>(aabb_max.x, aabb_min.y, aabb_max.z, 1.0),
    //     vec4<f32>(aabb_max.x, aabb_max.y, aabb_min.z, 1.0),
    //     vec4<f32>(aabb_max.x, aabb_max.y, aabb_max.z, 1.0)
    // );
    //
    // for (var i = 0; i < 6; i = i + 1) {
    //     var out = 0u;
    //
    //     for (var j = 0; j < 8; j = j + 1) {
    //         if (dot(view.planes[i], corners[j]) < 0.0) {
    //             out = out + 1u;
    //         }
    //     }
    //
    //     if (out == 8u) {
    //         // all points are outside the frustum -> cull
    //         return true;
    //     }
    // }
    //
    // return false;
}

fn outside_cull(tile: TileInfo) -> bool {
    // cull tiles outside of the terrain
    let local_position = vec2<f32>(tile.coords * tile.size) * view_config.tile_scale ;

    return local_position.x > f32(config.terrain_size) || local_position.y > f32(config.terrain_size);
}

fn cull(tile: TileInfo) -> bool {
    return outside_cull(tile) || frustum_cull(tile);
}

fn determine_lod(tile: TileInfo) -> u32 {
    let size = f32(tile.size) * view_config.tile_scale;
    let lod = u32(ceil(log2(size)));

    let center_position = (vec2<f32>(tile.coords) + 0.5) * size;

    let lookup = atlas_lookup(lod, center_position);
    let coords = lookup.atlas_coords * config.minmax_scale + config.minmax_offset;
    let lod = min(u32(lod), config.lod_count - 1u);

    let height = textureSampleLevel(height_atlas, terrain_sampler, coords, lookup.atlas_index, 0.0).x;

    var min_height = 1.0;
    var max_height = 0.0;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let offset = vec2<f32>(vec2<u32>((i & 1u), (i >> 1u & 1u))) - 0.5;

        let corner = vec2<i32>((coords + offset * size / node_size(lod)) * 132.0);

        let minmax = textureLoad(minmax_atlas, corner, lookup.atlas_index, 0).xy;

        min_height = min(min_height, minmax.x);
        max_height = max(max_height, minmax.y);
    }

    let min_position = vec4<f32>(center_position.x, min_height * config.height, center_position.y, 1.0);
    let max_position = vec4<f32>(center_position.x, max_height * config.height, center_position.y, 1.0);

    let min_screen = view.view_proj * min_position;
    let min_screen = min_screen.xy / min_screen.w;
    let max_screen = view.view_proj * max_position;
    let max_screen = max_screen.xy / max_screen.w;

    var dist = max_screen - min_screen; // * vec2<f32>(1920.0, 1080.0);
    dist = vec2<f32>(dist.x * 16.0 / 9.0, dist.y);

    let error = length(dist) * 200.0;


    // let error = (max_height - min_height) * config.height;
    // let error = error / size * 40.0;


    return u32(min(error, 0.999) * 4.0);
}

fn should_be_divided(tile: TileInfo) -> bool {
    if (tile.size == 1u) {
        return false;
    }

    var dist = 1000000000.0;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let corner_coords = vec2<u32>(tile.coords.x + (i       & 1u),
                                      tile.coords.y + (i >> 1u & 1u));

        let local_position = vec2<f32>(corner_coords * tile.size) * view_config.tile_scale;
        let world_position = approximate_world_position(local_position);
        dist = min(dist, distance(world_position, view.world_position.xyz, ));
    }

    // ADLOD might required small error toleranze.
    return dist < view_config.refinement_distance * f32(tile.size);
}

fn subdivide(tile: TileInfo) {
    let size = tile.size >> 1u;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let coords = vec2<u32>((tile.coords.x << 1u) + (i       & 1u),
                               (tile.coords.y << 1u) + (i >> 1u & 1u));

        let tile = TileInfo(coords, size);

        if (!cull(tile)) {
            temporary_tiles.data[child_index()] = tile;
        }
    }
}

fn finalise(tile: TileInfo) {
#ifdef ADAPTIVE
    let parent_tile  = TileInfo(tile.coords >> vec2<u32>(1u), tile.size << 1u);
    var lod          = determine_lod(tile);
    let parent_lod   = determine_lod(parent_tile);
    let count        = calc_tile_count(lod);
    let parent_count = calc_tile_count(parent_lod) >> 1u;

    if (count < parent_count) {
        // can not morph into parrent with the current lod, thus choose a lod that fits the parent count
        lod = ((parent_count + 1u) >> 1u) - 1u;
    }

    var counts        = count        << u32(4 * 6);
    var parent_counts = parent_count << u32(4 * 6);

    var directions = array<vec2<i32>, 4>(
        vec2<i32>(-1,  0),
        vec2<i32>( 0, -1),
        vec2<i32>( 1,  0),
        vec2<i32>( 0,  1)
    );

    for (var i = 0; i < 4; i = i + 1) {
        let neighbour_tile = TileInfo(vec2<u32>(vec2<i32>(tile.coords) + directions[i]), tile.size);
        let neighbour_parent_tile = TileInfo(neighbour_tile.coords >> vec2<u32>(1u), parent_tile.size);

        let edge_count        = calc_tile_count(determine_lod(neighbour_tile));
        let edge_parent_count = calc_tile_count(determine_lod(neighbour_parent_tile)) >> 1u;

        counts        = counts        | min(count,        edge_count)        << u32(i * 6);
        parent_counts = parent_counts | min(parent_count, edge_parent_count) << u32(i * 6);
    }

    let tile = Tile(tile.coords, tile.size, counts, parent_counts, 0u);
#else
    let lod = 0u;
    let tile = Tile(tile.coords, tile.size, 0u, 0u, 0u);
#endif

    final_tiles.data[final_index(lod)] = tile;
}

@compute @workgroup_size(64, 1, 1)
fn refine_tiles(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    if (invocation_id.x >= parameters.refinement_count) {
        return;
    }

    let tile = temporary_tiles.data[parent_index(invocation_id.x)];

    if (should_be_divided(tile)) {
        subdivide(tile);
    }
    else {
        finalise(tile);
    }
}
