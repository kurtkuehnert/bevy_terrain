#import bevy_terrain::types TerrainConfig, TerrainViewConfig, Tile, TileList, Parameters, NodeLookup
#import bevy_terrain::functions calculate_sphere_position, approximate_world_position, tile_coordinate, tile_local_position
#import bevy_terrain::bindings config

struct CullingData {
    world_position: vec4<f32>,
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    planes: array<vec4<f32>, 5>,
}

@group(0) @binding(0)
var<uniform> view: CullingData;

@group(1) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(1) @binding(1)
var quadtree: texture_2d_array<u32>;
@group(1) @binding(2)
var<storage, read_write> final_tiles: TileList;
@group(1) @binding(3)
var<storage, read_write> temporary_tiles: TileList;
@group(1) @binding(4)
var<storage, read_write> parameters: Parameters;

// Todo: figure out how to remove this duplicate
fn morph_threshold_distance(tile: Tile) -> f32 {
    let size = length(tile.u);

    #ifdef SPHERICAL
        let threshold_distance = size * config.radius * view_config.view_distance;
    #else
        let threshold_distance = size * config.terrain_size * view_config.view_distance;
    #endif

    return threshold_distance;
}

fn child_index() -> i32 {
    return atomicAdd(&parameters.child_index, parameters.counter);
}

fn parent_index(id: u32) -> i32 {
    return i32(view_config.tile_count - 1u) * clamp(parameters.counter, 0, 1) - i32(id) * parameters.counter;
}

fn final_index() -> i32 {
    return atomicAdd(&parameters.final_index, 1);
}

fn should_be_divided(tile: Tile) -> bool {
    var minimal_viewer_distance = 3.40282347E+38; // f32::MAX

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let local_position = tile_local_position(tile, vec2<f32>(f32(i & 1u), f32(i >> 1u & 1u)));
        let world_position = approximate_world_position(local_position);
        let viewer_distance = distance(world_position.xyz, view.world_position.xyz);

        minimal_viewer_distance = min(minimal_viewer_distance, viewer_distance);
    }

    return minimal_viewer_distance < morph_threshold_distance(tile);
}

fn subdivide(tile: Tile) {
    let child_u = 0.5 * tile.u;
    let child_v = 0.5 * tile.v;

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let uv = 0.5 * vec2<f32>(f32(i & 1u), f32(i >> 1u & 1u));
        let child_coordinate = tile_coordinate(tile, uv);

        let child_tile = Tile(child_coordinate, child_u, child_v, tile.side);

        temporary_tiles.data[child_index()] = child_tile;
    }
}

@compute @workgroup_size(64, 1, 1)
fn refine_tiles(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    if (invocation_id.x >= parameters.tile_count) {
        return;
    }

    let tile = temporary_tiles.data[parent_index(invocation_id.x)];

    if (should_be_divided(tile)) {
        subdivide(tile);
    }
    else {
        final_tiles.data[final_index()] = tile;
    }
}
