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

const min_height: f32 = 10.0 * -12000.0;
const max_height: f32 = 10.0 * 9000.0;


fn frustum_cull(tile: TileCoordinate) -> bool {
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

fn basic_horizon_cull(tile: TileCoordinate) -> bool {
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

    let o = position_local_to_world(vec3<f32>(0.0));
    let v = culling_view.world_position;
    let b = center_position;

    let r_o = 6371000.0;
    let r_b = radius;

    let ox = r_o - r_b;
    let ob = length(b - o);
    let vo = length(o - v);
    let vb = length(b - v);
    let vh = sqrt(vo * vo - r_o * r_o);
    let hy = sqrt(ob * ob - ox * ox);
    let vy = vh + hy;

    let vb_a_2 = vb * vb;
    let vb_e_2 = vy * vy + r_b * r_b;

    return vb_a_2 > vb_e_2;
}

const MAJOR_AXES: f32 = 6371000.0;
const MINOR_AXES: f32 = 6371000.0 / 2.0;

fn horizon_cull(tile: TileCoordinate) -> bool {

    // min height should be set to the minimal height of the tile adjacent to the edge point
    // we assume a continuous surface, thus the minimum of adjacent tile should be similar to this tile
    // thus, we can set the min and max height on a per tile basis


//    let radius = MINOR_AXES;
//    let min_radius = radius + min_height;
//    let factor = (MAJOR_AXES/ MINOR_AXES);
//
//    // transform positions to unit sphere
//    let ellipsoid_to_sphere = vec3<f32>(min_radius * factor, min_radius, min_radius * factor);

    let radius = MAJOR_AXES;
    let min_radius = radius + min_height;
    let factor = 1.0; // (MAJOR_AXES/ MINOR_AXES);
    let scaled_max_height = max_height / radius;

    // transform positions to unit sphere
    let ellipsoid_to_sphere = vec3<f32>(min_radius, min_radius/ factor, min_radius);


    // view position
    let v = culling_view.world_position / ellipsoid_to_sphere;

    // terrain origin
    let o = position_local_to_world(vec3<f32>(0.0)) / ellipsoid_to_sphere;

    // center of tile
    let center_c = Coordinate(tile.face, tile.lod, tile.xy, vec2<f32>(0.5));
    let center_l = compute_local_position(center_c);
    let c = position_local_to_world(center_l) / ellipsoid_to_sphere;

    // position on the edge of the tile closest to the viewer
    let edge_c = compute_subdivision_coordinate(center_c);
    let edge_l = compute_local_position(edge_c);
    let edge_n = compute_local_position(edge_c);
    let e = position_local_to_world(edge_l) / ellipsoid_to_sphere;

    // position closest to the viewer with maximum height applied
    // serves as a conservative ocluder proxy
    // if this point is not visible, no other point of the tile should be visible
    let t = e + scaled_max_height * normalize(e - o);

    let vt = t - v;
    let vo = o - v;
    let vo_vo = dot(vo, vo);
    let vo_vt = dot(vo, vt);
    let vt_vt = dot(vt, vt);

    // test if t is in front of the horizon plane
    if (vo_vt < vo_vo - 1) { return false; }

    // test if t is inside the horizon cone
    if (vo_vt * vo_vt / vt_vt > vo_vo - 1) { return true; }

    return false;
}

fn cull(tile: TileCoordinate) -> bool {
    // if (frustum_cull(tile)) { return true; }
    if (horizon_cull(tile)) { return true; }

    return false;
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
