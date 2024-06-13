#import bevy_terrain::types::Tile
#import bevy_terrain::bindings::{culling_view, view_config, final_tiles, temporary_tiles, parameters}
#import bevy_terrain::functions::{compute_coordinate, compute_local_position, local_to_world_position, tile_size, compute_relative_coordinate, compute_relative_position}

fn child_index() -> i32 {
    return atomicAdd(&parameters.child_index, parameters.counter);
}

fn parent_index(id: u32) -> i32 {
    return i32(view_config.tile_count - 1u) * clamp(parameters.counter, 0, 1) - i32(id) * parameters.counter;
}

fn final_index() -> i32 {
    return atomicAdd(&parameters.final_index, 1);
}

fn compute_corner_view_distance(tile: Tile, offset: vec2<f32>) -> f32 {
    let corner_coordinate     = compute_coordinate(tile, offset);
    let corner_local_position = compute_local_position(corner_coordinate);
    let corner_world_position = local_to_world_position(corner_local_position);
    var corner_view_distance  = distance(corner_world_position + view_config.approximate_height * corner_local_position, culling_view.world_position);

    if (corner_view_distance < view_config.precision_threshold_distance) {
        let corner_relative_coordinate = compute_relative_coordinate(tile, offset);
        let corner_relative_position   = compute_relative_position(corner_relative_coordinate);
        corner_view_distance           = length(corner_relative_position + view_config.approximate_height * corner_local_position);
    }

    return corner_view_distance;
}

fn should_be_divided(tile: Tile) -> bool {
    var min_view_distance = 3.40282347E+38; // f32::MAX

    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let corner_view_distance = compute_corner_view_distance(tile, vec2<f32>(f32(i & 1u), f32(i >> 1u & 1u)));
        min_view_distance = min(min_view_distance, corner_view_distance);
    }

    return min_view_distance < view_config.morph_distance * tile_size(tile.lod) * 6371000.0;
}

fn subdivide(tile: Tile) {
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let child_xy  = vec2<u32>((tile.xy.x << 1u) + (i & 1u), (tile.xy.y << 1u) + (i >> 1u & 1u));
        let child_lod = tile.lod + 1u;

        temporary_tiles[child_index()] = Tile(tile.side, child_lod, child_xy);
    }
}

@compute @workgroup_size(64, 1, 1)
fn refine_tiles(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    if (invocation_id.x >= parameters.tile_count) { return; }

    let tile = temporary_tiles[parent_index(invocation_id.x)];

    if (should_be_divided(tile)) {
        subdivide(tile);
    } else {
        final_tiles[final_index()] = tile;
    }
}
