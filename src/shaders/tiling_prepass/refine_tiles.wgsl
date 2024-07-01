#import bevy_terrain::types::Tile
#import bevy_terrain::bindings::{config, culling_view, view_config, final_tiles, temporary_tiles, parameters, terrain_model_approximation}
#import bevy_terrain::functions::{compute_local_position, compute_relative_position, position_local_to_world, normal_local_to_world, tile_count, tile_offset}

fn child_index() -> i32 {
    return atomicAdd(&parameters.child_index, parameters.counter);
}

fn parent_index(id: u32) -> i32 {
    return i32(view_config.tile_count - 1u) * clamp(parameters.counter, 0, 1) - i32(id) * parameters.counter;
}

fn final_index() -> i32 {
    return atomicAdd(&parameters.final_index, 1);
}

fn compute_view_distance(tile: Tile, offset: vec2<f32>) -> f32 {
    let local_position = compute_local_position(tile, offset);
    let world_position = position_local_to_world(local_position);
    let world_normal   = normal_local_to_world(local_position);
    var view_distance  = distance(world_position + view_config.approximate_height * world_normal, culling_view.world_position);

#ifdef TEST1
    if (view_distance < view_config.precision_threshold_distance) {
        let relative_position = compute_relative_position(tile, offset);
        view_distance         = length(relative_position + view_config.approximate_height * world_normal);
    }
#endif

    return view_distance;
}

fn should_be_divided(tile: Tile) -> bool {
    let offset = tile_offset(tile);

    let view_distance = compute_view_distance(tile, offset);
    // Todo: add some small tolerance, since vertices close to the offset could potentially be closer to the camera
    //       due to height differences
    let threshold_distance = 0.5 * view_config.morph_distance / tile_count(tile.lod);

    return view_distance < threshold_distance;
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
