#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings::{mesh, config, origins, view_config, tiles, quadtree, terrain_model_approximation}
#import bevy_terrain::types::{Tile, Quadtree, QuadtreeEntry, NodeLookup, Blend, BestLookup, Coordinate, Morph}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_render::maths::{affine3_to_square, mat2x4_f32_to_mat3x3_unpack}

const F0 = 0u;
const F1 = 1u;
const PS = 2u;
const PT = 3u;
const C_SQR = 0.87 * 0.87;

#ifdef SPHERICAL
const SIDE_COUNT = 6u;
#else
const SIDE_COUNT = 1u;
#endif

fn sphere_to_cube(xy: vec2<f32>) -> vec2<f32> {
    var uv: vec2<f32>;

    // s2 quadtratic as per https://docs.s2cell.aliddell.com/en/stable/s2_concepts.html#st
    // if (xy.x > 0.0) { uv.x =       0.5 * sqrt(1.0 + 3.0 * xy.x); }
    // else            { uv.x = 1.0 - 0.5 * sqrt(1.0 - 3.0 * xy.x); }
//
    // if (xy.y > 0.0) { uv.y =       0.5 * sqrt(1.0 + 3.0 * xy.y); }
    // else            { uv.y = 1.0 - 0.5 * sqrt(1.0 - 3.0 * xy.y); }

    // algebraic sigmoid c = 0.87 as per https://marlam.de/publications/cubemaps/lambers2019cubemaps.pdf
    uv = xy * sqrt((1.0 + C_SQR) / (1.0 + C_SQR * xy * xy));
    uv = 0.5 * xy + 0.5;

    return uv;
}

fn cube_to_sphere(uv: vec2<f32>) -> vec2<f32> {
    var xy: vec2<f32>;

    // s2 quadtratic as per https://docs.s2cell.aliddell.com/en/stable/s2_concepts.html#st
    // if (uv.x > 0.5) { xy.x =       (4.0 * pow(uv.x, 2.0) - 1.0) / 3.0; }
    // else            { xy.x = (1.0 - 4.0 * pow(1.0 - uv.x, 2.0)) / 3.0; }
//
    // if (uv.y > 0.5) { xy.y =       (4.0 * pow(uv.y, 2.0) - 1.0) / 3.0; }
    // else            { xy.y = (1.0 - 4.0 * pow(1.0 - uv.y, 2.0)) / 3.0; }

    // algebraic sigmoid c = 0.87 as per https://marlam.de/publications/cubemaps/lambers2019cubemaps.pdf

    xy = (uv - 0.5) / 0.5;
    xy = xy / sqrt(1.0 + C_SQR - C_SQR * xy * xy);

    return xy;
}

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
    let even_uv = vec2<f32>(vec2<u32>(coordinate.uv * view_config.grid_size) & vec2<u32>(~1u)) / view_config.grid_size;

    let target_lod  = log2(view_config.morph_distance / view_distance);
    let ratio       = select(inverse_mix(f32(coordinate.lod) + view_config.morph_range, f32(coordinate.lod), target_lod), 0.0, coordinate.lod == 0);

    return Coordinate(coordinate.side, coordinate.lod, coordinate.xy, mix(coordinate.uv, even_uv, ratio));
#else
    return coordinate;
#endif
}

fn compute_blend(view_distance: f32) -> Blend {
    let target_lod = min(log2(view_config.blend_distance / view_distance), f32(config.lod_count) - 0.00001);
    let lod        = u32(target_lod);

#ifdef BLEND
    let ratio = select(inverse_mix(f32(lod) + view_config.blend_range, f32(lod), target_lod), 0.0, lod == 0u);

    return Blend(lod, ratio);
#else
    return Blend(lod, 0.0);
#endif
}

fn compute_tile_uv(grid_index: u32) -> vec2<f32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;
    let tile_uv = vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);

    return vec2<f32>(tile_uv) / view_config.grid_size;
}

fn compute_coordinate(tile: Tile, uv: vec2<f32>) -> Coordinate {
#ifdef FRAGMENT
    return Coordinate(tile.side, tile.lod, tile.xy, uv, dpdx(uv), dpdy(uv));
#else
    return Coordinate(tile.side, tile.lod, tile.xy, uv);
#endif
}

fn compute_local_position(coordinate: Coordinate) -> vec3<f32> {
    let st = (vec2<f32>(coordinate.xy) + coordinate.uv) / tile_count(coordinate.lod);

#ifdef SPHERICAL
    let uv = cube_to_sphere(st);

    var local_position: vec3<f32>;

    switch (coordinate.side) {
        case 0u:      { local_position = vec3( -1.0, -uv.y,  uv.x); }
        case 1u:      { local_position = vec3( uv.x, -uv.y,   1.0); }
        case 2u:      { local_position = vec3( uv.x,   1.0,  uv.y); }
        case 3u:      { local_position = vec3(  1.0, -uv.x,  uv.y); }
        case 4u:      { local_position = vec3( uv.y, -uv.x,  -1.0); }
        case 5u:      { local_position = vec3( uv.y,  -1.0,  uv.x); }
        case default: {}
    }

    return normalize(local_position);
#else
    return vec3<f32>(st.x - 0.5, 0.0, st.y - 0.5);
#endif
}

fn compute_relative_position(coordinate: Coordinate) -> vec3<f32> {
    let params = terrain_model_approximation.sides[coordinate.side];

    let test        = coordinate_change_lod(coordinate, terrain_model_approximation.origin_lod);
    let relative_st = (vec2<f32>(vec2<i32>(test.xy) - params.view_xy) + test.uv - params.view_uv) / tile_count(terrain_model_approximation.origin_lod);

    let s = relative_st.x;
    let t = relative_st.y;
    let c = params.c;
    let c_s = params.c_s;
    let c_t = params.c_t;
    let c_ss = params.c_ss;
    let c_st = params.c_st;
    let c_tt = params.c_tt;

    return c + c_s * s + c_t * t + c_ss * s * s + c_st * s * t + c_tt * t * t;
}

fn approximate_view_distance(coordinate: Coordinate, view_world_position: vec3<f32>) -> f32 {
    let local_position = compute_local_position(coordinate);
    var world_position = position_local_to_world(local_position);
    let world_normal   = normal_local_to_world(local_position);
    var view_distance  = distance(world_position + terrain_model_approximation.approximate_height * world_normal, view_world_position);

#ifdef TEST1
    if (view_distance < view_config.precision_threshold_distance) {
        let relative_position = compute_relative_position(coordinate);
        view_distance         = length(relative_position + terrain_model_approximation.approximate_height * world_normal);
    }
#endif

    return view_distance;
}

fn compute_subdivision_coordinate(coordinate: Coordinate) -> Coordinate {
    let params  = terrain_model_approximation.sides[coordinate.side];

#ifdef FRAGMENT
    var view_coordinate = Coordinate(coordinate.side, terrain_model_approximation.origin_lod, vec2<u32>(params.view_xy), params.view_uv, vec2<f32>(0.0), vec2<f32>(0.0));
#else
    var view_coordinate = Coordinate(coordinate.side, terrain_model_approximation.origin_lod, vec2<u32>(params.view_xy), params.view_uv);
#endif

    view_coordinate = coordinate_change_lod(view_coordinate, coordinate.lod);
    var offset = vec2<i32>(view_coordinate.xy) - vec2<i32>(coordinate.xy);
    var uv = view_coordinate.uv;

    if      (offset.x < 0) { uv.x = 0.0; }
    else if (offset.x > 0) { uv.x = 1.0; }
    if      (offset.y < 0) { uv.y = 0.0; }
    else if (offset.y > 0) { uv.y = 1.0; }

    var subdivision_coordinate = coordinate;
    subdivision_coordinate.uv = uv;
    return subdivision_coordinate;
}

// Todo: remove/replace this
fn tile_count(lod: u32) -> f32 { return f32(1u << lod); }
fn node_count(lod: u32) -> f32 { return f32(1u << lod); }

fn inside_square(position: vec2<f32>, origin: vec2<f32>, size: f32) -> f32 {
    let inside = step(origin, position) * step(position, origin + size);

    return inside.x * inside.y;
}

fn coordinate_change_lod(coordinate: Coordinate, new_lod: u32) -> Coordinate {
    var new_coordinate = coordinate;
    new_coordinate.lod = new_lod;

    let lod_difference = i32(coordinate.lod) - i32(new_lod);

    if (lod_difference < 0) {
        let size          = 1u << u32(-lod_difference);
        let scaled_uv     = coordinate.uv * f32(size);
        new_coordinate.xy = vec2<u32>(coordinate.xy * size) + vec2<u32>(scaled_uv);
        new_coordinate.uv = scaled_uv % 1.0;
        #ifdef FRAGMENT
            new_coordinate.uv_dx *= f32(size);
            new_coordinate.uv_dy *= f32(size);
        #endif
    } else {
        let size          = 1u << u32(lod_difference);
        new_coordinate.xy = vec2<u32>(coordinate.xy / size);
        new_coordinate.uv = (vec2<f32>(coordinate.xy % size) + coordinate.uv) / f32(size);
        #ifdef FRAGMENT
            new_coordinate.uv_dx /= f32(size);
            new_coordinate.uv_dy /= f32(size);
        #endif
    }

    return new_coordinate;
}

fn quadtree_uv(coordinate: Coordinate) -> vec2<f32> {
    let origin_xy = vec2<i32>(origins[coordinate.side * config.lod_count + coordinate.lod]);
    let quadtree_size = f32(min(view_config.quadtree_size, 1u << coordinate.lod));

    return (vec2<f32>(vec2<i32>(coordinate.xy) - origin_xy) + coordinate.uv) / quadtree_size;
}


fn lookup_quadtree_entry(coordinate: Coordinate) -> QuadtreeEntry {
    let quadtree_side  = coordinate.side;
    let quadtree_lod   = coordinate.lod;
    let quadtree_xy    = vec2<u32>(coordinate.xy) % view_config.quadtree_size;
    let quadtree_index = ((quadtree_side  * config.lod_count +
                           quadtree_lod)  * view_config.quadtree_size +
                           quadtree_xy.x) * view_config.quadtree_size +
                           quadtree_xy.y;

    return quadtree[quadtree_index];
}

fn lookup_best(lookup_coordinate: Coordinate) -> BestLookup {
    var coordinate: Coordinate; var quadtree_uv: vec2<f32>;

    var new_coordinate  = coordinate_change_lod(lookup_coordinate, 0u);
    var new_quadtree_uv = new_coordinate.uv;

    while (new_coordinate.lod < config.lod_count && !any(new_quadtree_uv < vec2<f32>(0.0)) && !any(new_quadtree_uv > vec2<f32>(1.0))) {
        coordinate  = new_coordinate;
        quadtree_uv = new_quadtree_uv;

        new_coordinate  = coordinate_change_lod(lookup_coordinate, new_coordinate.lod + 1u);
        new_quadtree_uv = quadtree_uv(new_coordinate);
    }

    let quadtree_entry = lookup_quadtree_entry(coordinate);

    coordinate = coordinate_change_lod(coordinate, quadtree_entry.atlas_lod);

    return BestLookup(NodeLookup(quadtree_entry.atlas_index, coordinate), quadtree_uv);
}

fn lookup_node(lookup_coordinate: Coordinate, blend: Blend, lod_offset: u32) -> NodeLookup {
#ifdef QUADTREE_LOD
    return lookup_best(lookup_coordinate).lookup;
#else
    var coordinate = coordinate_change_lod(lookup_coordinate, blend.lod - lod_offset);

    let quadtree_entry = lookup_quadtree_entry(coordinate);

    coordinate = coordinate_change_lod(coordinate, quadtree_entry.atlas_lod);

    return NodeLookup(quadtree_entry.atlas_index, coordinate);
#endif
}
