#import bevy_terrain::types::{TileCoordinate, Coordinate}
#import bevy_terrain::bindings::{terrain, culling_view, terrain_view, final_tiles, approximate_height_write, temporary_tiles, state}
#import bevy_terrain::functions::{approximate_view_distance, compute_local_position, compute_relative_position, position_local_to_world, normal_local_to_world, tile_count, compute_subdivision_coordinate}

fn child_index() -> i32 {
    return atomicAdd(&state.child_index, state.counter);
}

fn parent_index(id: u32) -> i32 {
    return i32(terrain_view.tile_count - 1u) * clamp(state.counter, 0, 1) - i32(id) * state.counter;
}

fn final_index() -> i32 {
    return atomicAdd(&state.final_index, 1);
}

fn should_be_divided(tile: TileCoordinate) -> bool {
    let coordinate    = compute_subdivision_coordinate(Coordinate(tile.face, tile.lod, tile.xy, vec2<f32>(0.0)));
    let view_distance = approximate_view_distance(coordinate, culling_view.world_position, approximate_height_write);

    return view_distance < terrain_view.subdivision_distance / tile_count(tile.lod + 1);
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
    let center_l = compute_local_position(Coordinate(tile.face, tile.lod, tile.xy, vec2<f32>(0.5)));
    let center = position_local_to_world(center_l);

    // identify furthest corner from center

    var radius = 0.0;

    // we have to build a bounding sphere using the four courners to be conservative
    // using the closest edge does not suffice
    for (var i = 0u; i < 4; i = i + 1) {
        let corner_uv = vec2<f32>(f32(i & 1u), f32(i >> 1u & 1u));
        let l = compute_local_position(Coordinate(tile.face, tile.lod, tile.xy, corner_uv));
        let w = position_local_to_world(l);
        let n = normal_local_to_world(l);
        let c_min = w + min_height * n;
        let c_max = w + max_height * n;

        // Consider both min and max height
        radius = max(radius, max(distance(center, c_min), distance(center, c_max)));
    }

    for (var i = 0; i < 6; i = i + 1) {
        let half_space = culling_view.half_spaces[i];

        if (dot(half_space, vec4<f32>(center, 1.0)) + radius < 0.0) {
            return true;
        }
    }

     return false;
}

const MAJOR_AXES: f32 = 6371000.0;
const MINOR_AXES: f32 = 6371000.0;

fn horizon_cull(tile: TileCoordinate) -> bool {
    if (tile.lod < 3) { return false; }
    // up to LOD 3, the closest point estimation is not reliable when projecting to adjacent sides
    // to prevent issues with cut of corners, horizon culling is skipped for those cases
    // this still leads to adeqate culling when close to the surface

    // min height should be set to the minimal height of the tile adjacent to the edge point
    // we assume a continuous surface, thus the minimum of adjacent tile should be similar to this tile
    // thus, we can set the min and max height on a per tile basis


    let radius = MAJOR_AXES;
    let aspect_ratio = (MAJOR_AXES/ MINOR_AXES);

    // transform from ellipsoidal to spherical coordinates
    // this eliminates the oblatness of the ellipsoid
    let ellipsoid_to_sphere = vec3<f32>(radius, radius/ aspect_ratio, radius);

    // radius of our culling sphere
    // for correct conservative beviour, we have to adjust the minimal height according to the aspect ratio
    let r = 1 + min_height * aspect_ratio / radius;

    // view position
    let v = culling_view.world_position / ellipsoid_to_sphere;

    // terrain origin
    let o = position_local_to_world(vec3<f32>(0.0)) / ellipsoid_to_sphere;

    // position on the edge of the tile closest to the viewer with maximum height applied
    // serves as a conservative ocluder proxy
    // if this point is not visible, no other point of the tile should be visible
    let edge_c = compute_subdivision_coordinate(Coordinate(tile.face, tile.lod, tile.xy, vec2<f32>(0.0)));
    let edge_l = compute_local_position(edge_c);
    let t = (position_local_to_world(edge_l) + max_height * normal_local_to_world(edge_l)) / ellipsoid_to_sphere;

    let vt = t - v;
    let vo = o - v;
    let r_r = r * r;
    let vo_vo = dot(vo, vo);
    let vo_vt = dot(vo, vt);
    let vt_vt = dot(vt, vt);

    // test if t is in front of the horizon plane
    if (vo_vt < vo_vo - r_r) { return false; }

    // test if t is inside the horizon cone
    if (vo_vt * vo_vt / vt_vt > vo_vo - r_r) { return true; }

    return false;
}

fn cull(tile: TileCoordinate) -> bool {
    if (frustum_cull(tile)) { return true; }
    if (horizon_cull(tile)) { return true; }

    return false;
}

@compute @workgroup_size(64, 1, 1)
fn refine_tiles(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    if (invocation_id.x >= state.tile_count) { return; }

    let tile = temporary_tiles[parent_index(invocation_id.x)];

    if cull(tile) { return; }

    if (should_be_divided(tile)) {
        subdivide(tile);
    } else {
        final_tiles[final_index()] = tile;
    }
}
