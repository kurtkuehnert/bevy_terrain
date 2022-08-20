#import bevy_terrain::types
#import bevy_terrain::parameters

// Todo: increase workgroup size

struct TerrainConfig {
    lod_count: u32,
    height: f32,
    chunk_size: u32,
    terrain_size: u32,
    height_scale: f32,
    density_scale: f32,
    _empty: u32,
    _empty: u32,
    height_offset: f32,
    density_offset: f32,
    _empty: u32,
    _empty: u32,
}

struct CullData {
    world_position: vec4<f32>,
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    planes: array<vec4<f32>, 6>,
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
var<uniform> view: CullData;

 // terrain bindings
@group(2) @binding(0)
var<uniform> config: TerrainConfig;
@group(2) @binding(1)
var filter_sampler: sampler;
@group(2) @binding(2)
var height_atlas: texture_2d_array<f32>;
@group(2) @binding(3)
var density_atlas: texture_2d_array<f32>;

#import bevy_terrain::atlas

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
        let plane = view.planes[i];

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

fn determine_lod(tile: TileInfo) -> u32 {
#ifdef DENSITY
    let local_position = (vec2<f32>(tile.coords) + 0.5) * f32(tile.size) * view_config.tile_scale;
    let log_distance = log2(f32(tile.size) * view_config.tile_scale);

    let lookup = atlas_lookup(log_distance, local_position);
    let slope = textureSampleLevel(density_atlas, filter_sampler, lookup.atlas_coords, lookup.atlas_index, 0.0).x;

    let slope = min(slope * 10.0, 0.999);

    return u32(slope * 4.0);
#else
    return 3u;
#endif
}

fn cull(tile: TileInfo) -> bool {
    // cull tiles outside of the terrain
    let local_position = vec2<f32>(tile.coords * tile.size) * view_config.tile_scale ;
    if (local_position.x > f32(config.terrain_size) || local_position.y > f32(config.terrain_size)) {
        return true;
    }

    // if (frustum_cull(local_position, config.tile_scale * f32(config.tile_size * size))) {
    //     return true;
    // }

    return false;
}


fn should_be_divided(tile: TileInfo) -> bool {
    if (tile.size == 2u) {
        return false;
    }

    var divide = false;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let corner_coords = vec2<u32>(tile.coords.x + (i       & 1u),
                                      tile.coords.y + (i >> 1u & 1u));

        let local_position = vec2<f32>(corner_coords * tile.size) * view_config.tile_scale;
        let world_position = approximate_world_position(local_position);
        let distance = length(view.world_position.xyz - world_position) * 0.99; // consider adding a small error mitigation

        divide = divide || (distance < f32(tile.size >> 1u) * view_config.view_distance);
    }

    return divide;
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

    final_tiles.data[final_index(lod)] = Tile(tile.coords, tile.size, counts, parent_counts, 0u);
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
