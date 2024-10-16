#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings::{mesh, terrain, origins, terrain_view, approximate_height, geometry_tiles, tile_tree}
#import bevy_terrain::types::{TileCoordinate, TileTree, TileTreeEntry, AtlasTile, Blend, BestLookup, Coordinate, Morph}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_render::maths::{affine3_to_square, mat2x4_f32_to_mat3x3_unpack}

const F0 = 0u;
const F1 = 1u;
const PS = 2u;
const PT = 3u;
const C_SQR = 0.87 * 0.87;

fn normal_local_to_world(local_position: vec3<f32>) -> vec3<f32> {
#ifdef SPHERICAL
    let local_normal = local_position;
#else
    let local_normal = vec3<f32>(0.0, 1.0, 0.0);
#endif

    let world_from_local = mat2x4_f32_to_mat3x3_unpack(mesh[0].local_from_world_transpose_a,
                                                       mesh[0].local_from_world_transpose_b);
    return normalize(world_from_local * local_normal);
}

fn position_local_to_world(local_position: vec3<f32>) -> vec3<f32> {
    let world_from_local = affine3_to_square(mesh[0].world_from_local);
    return (world_from_local * vec4<f32>(local_position, 1.0)).xyz;
}

fn inverse_mix(a: f32, b: f32, value: f32) -> f32 {
    return saturate((value - a) / (b - a));
}

fn compute_morph(coordinate: Coordinate, view_distance: f32) -> Coordinate {
#ifdef MORPH
    // Morphing more than one layer at once is not possible, since the approximate view distance for vertices that
    // should be placed on the same position will be slightly different, so the target lod and thus the ratio will be
    // slightly off as well, which results in a pop.
    let even_uv = vec2<f32>(vec2<u32>(coordinate.uv * terrain_view.grid_size) & vec2<u32>(~1u)) / terrain_view.grid_size;

    let target_lod  = log2(2.0 * terrain_view.morph_distance / view_distance);
    let ratio       = select(inverse_mix(f32(coordinate.lod) + terrain_view.morph_range, f32(coordinate.lod), target_lod), 0.0, coordinate.lod == 0);

    return Coordinate(coordinate.face, coordinate.lod, coordinate.xy, mix(coordinate.uv, even_uv, ratio));
#else
    return coordinate;
#endif
}

fn compute_blend(view_distance: f32) -> Blend {
    let target_lod = min(log2(terrain_view.blend_distance / view_distance), f32(terrain.lod_count) - 0.00001);
    let lod        = u32(target_lod);

#ifdef BLEND
    let ratio = select(inverse_mix(f32(lod) + terrain_view.blend_range, f32(lod), target_lod), 0.0, lod == 0u);

    return Blend(lod, ratio);
#else
    return Blend(lod, 0.0);
#endif
}

fn compute_tile_uv(vertex_index: u32) -> vec2<f32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let grid_index   = vertex_index % terrain_view.vertices_per_tile;
    let row_index    = clamp(grid_index % terrain_view.vertices_per_row, 1u, terrain_view.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / terrain_view.vertices_per_row;

    return vec2<f32>(f32(column_index + (row_index & 1u)), f32(row_index >> 1u)) / terrain_view.grid_size;
}

fn compute_local_position(coordinate: Coordinate) -> vec3<f32> {
    let uv = (vec2<f32>(coordinate.xy) + coordinate.uv) / tile_count(coordinate.lod);

#ifdef SPHERICAL
    let xy = (2.0 * uv - 1.0) / sqrt(1.0 - 4.0 * C_SQR * (uv - 1.0) * uv);

    // this is faster than the CPU SIDE_MATRICES approach
    var local_position: vec3<f32>;
    switch (coordinate.face) {
        case 0u:      { local_position = vec3( -1.0, -xy.y,  xy.x); }
        case 1u:      { local_position = vec3( xy.x, -xy.y,   1.0); }
        case 2u:      { local_position = vec3( xy.x,   1.0,  xy.y); }
        case 3u:      { local_position = vec3(  1.0, -xy.x,  xy.y); }
        case 4u:      { local_position = vec3( xy.y, -xy.x,  -1.0); }
        case 5u:      { local_position = vec3( xy.y,  -1.0,  xy.x); }
        case default: {}
    }

    return normalize(local_position);
#else
    return vec3<f32>(uv.x - 0.5, 0.0, uv.y - 0.5);
#endif
}

#ifdef HIGH_PRECISION
fn compute_relative_position(coordinate: Coordinate) -> vec3<f32> {
    let view_coordinate = compute_view_coordinate(coordinate.face, coordinate.lod);

    let relative_uv = (vec2<f32>(vec2<i32>(coordinate.xy) - vec2<i32>(view_coordinate.xy)) + coordinate.uv - view_coordinate.uv) / tile_count(coordinate.lod);
    let u = relative_uv.x;
    let v = relative_uv.y;

    let approximation = terrain_view.surface_approximation[coordinate.face];
    let c = approximation.c;
    let c_u = approximation.c_u;
    let c_v = approximation.c_v;
    let c_uu = approximation.c_uu;
    let c_uv = approximation.c_uv;
    let c_vv = approximation.c_vv;

    return c + c_u * u + c_v * v + c_uu * u * u + c_uv * u * v + c_vv * v * v;
}
#endif

fn approximate_view_distance(coordinate: Coordinate, view_world_position: vec3<f32>) -> f32 {
    let local_position = compute_local_position(coordinate);
    var world_position = position_local_to_world(local_position);
    let world_normal   = normal_local_to_world(local_position);
    var view_distance  = distance(world_position + approximate_height * world_normal, view_world_position);

#ifdef HIGH_PRECISION
    if (view_distance < terrain_view.precision_threshold_distance) {
        let relative_position = compute_relative_position(coordinate);
        view_distance         = length(relative_position + approximate_height * world_normal);
    }
#endif

    return view_distance;
}

fn compute_subdivision_coordinate(coordinate: Coordinate) -> Coordinate {
    let view_coordinate = compute_view_coordinate(coordinate.face, coordinate.lod);

    var offset = vec2<i32>(view_coordinate.xy) - vec2<i32>(coordinate.xy);
    var uv     = view_coordinate.uv;

    if      (offset.x < 0) { uv.x = 0.0; }
    else if (offset.x > 0) { uv.x = 1.0; }
    if      (offset.y < 0) { uv.y = 0.0; }
    else if (offset.y > 0) { uv.y = 1.0; }

    var subdivision_coordinate = coordinate;
    subdivision_coordinate.uv = uv;
    return subdivision_coordinate;
}

fn tile_count(lod: u32) -> f32 { return f32(1u << lod); }

fn inside_square(position: vec2<f32>, origin: vec2<f32>, size: f32) -> f32 {
    let inside = step(origin, position) * step(position, origin + size);

    return inside.x * inside.y;
}

fn coordinate_change_lod(coordinate: ptr<function, Coordinate>, new_lod: u32) {
    let lod_difference = i32(new_lod) - i32((*coordinate).lod);

    if (lod_difference == 0) { return; }

    let delta_count = 1u << u32(abs(lod_difference));
    let delta_size  = pow(2.0, f32(lod_difference));

    (*coordinate).lod = new_lod;

    if (lod_difference > 0) {
        let scaled_uv    = (*coordinate).uv * delta_size;
        (*coordinate).xy = (*coordinate).xy * delta_count + vec2<u32>(scaled_uv);
        (*coordinate).uv = scaled_uv % 1.0;
    } else {
        let xy = (*coordinate).xy;
        (*coordinate).xy = xy / delta_count;
        (*coordinate).uv = (vec2<f32>(xy % delta_count) + (*coordinate).uv) * delta_size;
    }

#ifdef FRAGMENT
    (*coordinate).uv_dx *= delta_size;
    (*coordinate).uv_dy *= delta_size;
#endif
}

fn compute_view_coordinate(face: u32, lod: u32) -> Coordinate {
    let coordinate = terrain_view.view_coordinates[face];

#ifdef FRAGMENT
    var view_coordinate = Coordinate(face, terrain_view.view_lod, coordinate.xy, coordinate.uv, vec2<f32>(0.0), vec2<f32>(0.0));
#else
    var view_coordinate = Coordinate(face, terrain_view.view_lod, coordinate.xy, coordinate.uv);
#endif

    coordinate_change_lod(&view_coordinate, lod);

    return view_coordinate;
}

fn compute_tile_tree_uv(coordinate: Coordinate) -> vec2<f32> {
    let view_coordinate = compute_view_coordinate(coordinate.face, coordinate.lod);

    let tile_count = i32(tile_count(coordinate.lod));
    let tree_size  = min(i32(terrain_view.tree_size), tile_count);
    let tree_xy    = vec2<i32>(view_coordinate.xy) + vec2<i32>(round(view_coordinate.uv)) - vec2<i32>(terrain_view.tree_size / 2);
    let view_xy  = clamp(tree_xy, vec2<i32>(0), vec2<i32>(tile_count - tree_size));

    return (vec2<f32>(vec2<i32>(coordinate.xy) - view_xy) + coordinate.uv) / f32(tree_size);
}


fn lookup_tile_tree_entry(coordinate: Coordinate) -> TileTreeEntry {
    let tree_xy    = vec2<u32>(coordinate.xy) % terrain_view.tree_size;
    let tree_index = ((coordinate.face * terrain.lod_count +
                       coordinate.lod) * terrain_view.tree_size +
                       tree_xy.x)      * terrain_view.tree_size +
                       tree_xy.y;

    return tile_tree[tree_index];
}

// Todo: implement this more efficiently
fn lookup_best(lookup_coordinate: Coordinate) -> BestLookup {
    var coordinate: Coordinate; var tile_tree_uv: vec2<f32>;

    var new_coordinate   = lookup_coordinate;
    coordinate_change_lod(&new_coordinate , 0u);
    var new_tile_tree_uv = new_coordinate.uv;

    while (new_coordinate.lod < terrain.lod_count && !any(new_tile_tree_uv <= vec2<f32>(0.0)) && !any(new_tile_tree_uv >= vec2<f32>(1.0))) {
        coordinate  = new_coordinate;
        tile_tree_uv = new_tile_tree_uv;

        new_coordinate = lookup_coordinate;
        coordinate_change_lod(&new_coordinate, coordinate.lod + 1u);
        new_tile_tree_uv = compute_tile_tree_uv(new_coordinate);
    }

    let tile_tree_entry = lookup_tile_tree_entry(coordinate);

    coordinate_change_lod(&coordinate, tile_tree_entry.atlas_lod);

    return BestLookup(AtlasTile(tile_tree_entry.atlas_index, coordinate), tile_tree_uv);
}

fn lookup_tile(lookup_coordinate: Coordinate, blend: Blend, lod_offset: u32) -> AtlasTile {
#ifdef TILE_TREE_LOD
    return lookup_best(lookup_coordinate).tile;
#else
    var coordinate = lookup_coordinate;

    coordinate_change_lod(&coordinate, blend.lod - lod_offset);

    let tile_tree_entry = lookup_tile_tree_entry(coordinate);

    coordinate_change_lod(&coordinate, tile_tree_entry.atlas_lod);

    return AtlasTile(tile_tree_entry.atlas_index, coordinate);
#endif
}
