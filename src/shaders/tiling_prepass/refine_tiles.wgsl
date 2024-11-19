#import bevy_terrain::types::{TileCoordinate, Coordinate}
#import bevy_terrain::bindings::{terrain, culling_view, terrain_view, final_tiles, temporary_tiles, parameters}
#import bevy_terrain::functions::{approximate_view_distance, compute_local_position, compute_relative_position, position_local_to_world, normal_local_to_world, tile_count, compute_subdivision_coordinate}

fn child_index() -> i32 {
    return atomicAdd(&parameters.child_index, parameters.counter);
}

fn parent_index(id: u32) -> i32 {
    return i32(terrain_view.tile_count - 1u) * clamp(parameters.counter, 0, 1) - i32(id) * parameters.counter;
}

fn final_index() -> i32 {
    return atomicAdd(&parameters.final_index, 1);
}

fn should_be_divided(tile: TileCoordinate) -> bool {
    let coordinate    = compute_subdivision_coordinate(Coordinate(tile.face, tile.lod, tile.xy, vec2<f32>(0.0)));
    let view_distance = approximate_view_distance(coordinate, culling_view.world_position);

    return view_distance < terrain_view.subdivision_distance / tile_count(tile.lod);
}

fn subdivide(tile: TileCoordinate) {
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let child_xy  = vec2<u32>((tile.xy.x << 1u) + (i & 1u), (tile.xy.y << 1u) + (i >> 1u & 1u));
        let child_lod = tile.lod + 1u;

        temporary_tiles[child_index()] = TileCoordinate(tile.face, child_lod, child_xy);
    }
}

fn frustum_cull(tile: TileCoordinate) -> bool {
    let max_height = 9000.0;

    let tile_uv = vec2<f32>(0.5);

    let center_c = Coordinate(tile.face, tile.lod, tile.xy, tile_uv);
    let center_l = compute_local_position(center_c);
    let center_position = position_local_to_world(center_l);

    // identify furthest corner from center

    var radius = 0.0;

    for (var i = 0u; i < 4; i = i + 1) {
        let corner_uv = vec2<f32>(f32(i & 1u), f32(i >> 1u & 1u));
        let c               = Coordinate(tile.face, tile.lod, tile.xy, corner_uv);
        let l               = compute_local_position(c);
        let corner_position = position_local_to_world(l) + max_height * normal_local_to_world(l);

        // Consider both min and max height
        radius = max(radius, distance(center_position, corner_position));
    }

    let center = vec4<f32>(center_position, 1.0);

    for (var i = 0; i < 5; i = i + 1) {
        let half_space = culling_view.half_spaces[i];

        if (dot(half_space, center) + radius <= 0.0) {
            return true;
        }
    }

     return false;
}

fn cull(tile: TileCoordinate) -> bool {
//    if (tile.lod > 2) {
//        return true;
//    }

    return frustum_cull(tile);
}

@compute @workgroup_size(64, 1, 1)
fn refine_tiles(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    if (invocation_id.x >= parameters.tile_count) { return; }

    let tile = temporary_tiles[parent_index(invocation_id.x)];

    if cull(tile) {
        return;
    }

    if (should_be_divided(tile)) {
        subdivide(tile);
    } else {
        final_tiles[final_index()] = tile;
    }
}
