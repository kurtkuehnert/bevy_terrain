#import bevy_terrain::types::{TerrainConfig, TerrainViewConfig, Tile, TileList, Parameters, NodeLookup, UVCoordinate}
#import bevy_terrain::bindings::config
#import bevy_terrain::functions::{local_position_from_coordinate, tile_coordinate, tile_size}

struct CullingData {
    world_position: vec3<f32>,
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
    return view_config.morph_distance * tile_size(tile.lod);
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
    var min_view_distance = 3.40282347E+38; // f32::MAX

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let corner_coordinate = tile_coordinate(tile, vec2<f32>(f32(i & 1u), f32(i >> 1u & 1u)));
        let corner_local_position = local_position_from_coordinate(corner_coordinate, view_config.approximate_height);
        let corner_view_distance = distance(corner_local_position, view_config.view_local_position);

        min_view_distance = min(min_view_distance, corner_view_distance);
    }

    return min_view_distance < morph_threshold_distance(tile);
}

fn subdivide(tile: Tile) {
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let child_xy  = vec2<u32>((tile.xy.x << 1u) + (i & 1u), (tile.xy.y << 1u) + (i >> 1u & 1u));
        let child_lod = tile.lod + 1u;

        temporary_tiles.data[child_index()] = Tile(tile.side, child_lod, child_xy);
    }
}

@compute @workgroup_size(64, 1, 1)
fn refine_tiles(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    if (invocation_id.x >= parameters.tile_count) { return; }

    let tile = temporary_tiles.data[parent_index(invocation_id.x)];

    if (should_be_divided(tile)) {
        subdivide(tile);
    } else {
        final_tiles.data[final_index()] = tile;
    }
}
