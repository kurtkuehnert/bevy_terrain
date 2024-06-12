#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings::{config, view_config, tiles, quadtree, model_view_approximation}
#import bevy_terrain::types::{Tile, Quadtree, NodeLookup, Blend, LookupInfo, Coordinate, Morph}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh
#import bevy_render::maths::affine_to_square

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

fn local_to_world_position(local_position: vec3<f32>) -> vec3<f32> {
    return (affine_to_square(mesh[0].model) * vec4<f32>(local_position, 1.0)).xyz;
}

fn world_to_clip_position(world_position: vec3<f32>) -> vec4<f32> {
    return view.view_proj * vec4<f32>(world_position, 1.0);
}

fn compute_morph(view_distance: f32, lod: u32, grid_offset: vec2<f32>) -> Morph {
    let threshold_distance = 2.0 * view_config.morph_distance * tile_size(lod) * 6371000.0;
    let ratio = clamp(1.0 - (1.0 - view_distance / threshold_distance) / view_config.morph_range, 0.0, 1.0);

    let even_offset = vec2<f32>(vec2<u32>(grid_offset * view_config.grid_size) & vec2<u32>(4294967294u)) / view_config.grid_size;
    let offset = mix(grid_offset, even_offset, ratio);

    return Morph(offset, ratio);
}

fn compute_blend(view_distance: f32) -> Blend {
    let lod_f32 = log2(2.0 * view_config.blend_distance / view_distance * 6371000.0);
    let lod     = clamp(u32(lod_f32), 0u, config.lod_count - 1u);

#ifdef BLEND
    let ratio = select(1.0 - (lod_f32 % 1.0) / view_config.blend_range, 0.0, lod_f32 < 1.0 || lod_f32 > f32(config.lod_count));
#else
    let ratio = 0.0;
#endif

    return Blend(lod, ratio);
}

fn compute_grid_offset(grid_index: u32) -> vec2<f32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;
    let offset = vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);

    return vec2<f32>(offset) / view_config.grid_size;
}

fn compute_coordinate(tile: Tile, offset: vec2<f32>) -> Coordinate {
    return Coordinate(tile.side, (vec2<f32>(tile.xy) + offset) * tile_size(tile.lod));
}

fn compute_local_position(coordinate: Coordinate) -> vec3<f32> {
#ifdef SPHERICAL
    let xy = cube_to_sphere(coordinate.uv);

    var local_position: vec3<f32>;

    switch (coordinate.side) {
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
    return vec3<f32>(coordinate.uv.x - 0.5, 0.0, coordinate.uv.y - 0.5);
#endif
}

fn compute_relative_coordinate(tile: Tile, grid_offset: vec2<f32>) -> Coordinate {
    let side = model_view_approximation.sides[tile.side];

    let lod_difference = tile.lod - u32(model_view_approximation.origin_lod);
    let origin_xy = vec2<i32>(side.origin_xy.x << lod_difference, side.origin_xy.y << lod_difference);
    let tile_offset = vec2<i32>(tile.xy) - origin_xy;
    let relative_st = (vec2<f32>(tile_offset) + grid_offset) * tile_size(tile.lod) + side.delta_relative_st;

    return Coordinate(tile.side, relative_st);
}

fn compute_relative_position(relative_coordinate: Coordinate) -> vec3<f32> {
    let params = model_view_approximation.sides[relative_coordinate.side];

    let s = relative_coordinate.uv.x;
    let t = relative_coordinate.uv.y;
    let c = params.c;
    let c_s = params.c_s;
    let c_t = params.c_t;
    let c_ss = params.c_ss;
    let c_st = params.c_st;
    let c_tt = params.c_tt;

    return c + c_s * s + c_t * t + c_ss * s * s + c_st * s * t + c_tt * t * t;
}

fn tile_size(lod: u32) -> f32 {
    return 1.0 / f32(1u << lod);
}

fn node_count(lod: u32) -> f32 {
    return f32(1u << lod);
}

fn node_coordinate(coordinate: Coordinate, lod: u32) -> vec2<f32> {
    return min(coordinate.uv, vec2(0.9999999)) * node_count(lod);
}

fn inside_square(position: vec2<f32>, origin: vec2<f32>, size: f32) -> f32 {
    let inside = step(origin, position) * step(position, origin + size);

    return inside.x * inside.y;
}

fn quadtree_origin(quadtree_coordinate: Coordinate, lod: u32) -> vec2<f32> {
    let node_coordinate = node_coordinate(quadtree_coordinate, lod);
    let max_offset      = node_count(lod) - f32(view_config.quadtree_size);

    return clamp(round(node_coordinate - 0.5 * f32(view_config.quadtree_size)), vec2<f32>(0.0), vec2<f32>(max_offset));
}


fn lookup_node(info: LookupInfo, lod_offset: u32) -> NodeLookup {
    let quadtree_lod           = info.lod - lod_offset;
    let quadtree_side          = info.coordinate.side;
    let quadtree_coordinate    = vec2<u32>(node_coordinate(info.coordinate, quadtree_lod)) % view_config.quadtree_size;
    let quadtree_index         = (((SIDE_COUNT                + quadtree_side        ) *
                                    config.lod_count          + quadtree_lod         ) *
                                    view_config.quadtree_size + quadtree_coordinate.x) *
                                    view_config.quadtree_size + quadtree_coordinate.y;
    let quadtree_entry         = quadtree[quadtree_index];

    var lookup: NodeLookup;
    lookup.lod                 = quadtree_entry.atlas_lod;
    lookup.index               = quadtree_entry.atlas_index;
    lookup.coordinate          = node_coordinate(info.coordinate, lookup.lod) % 1.0;
    lookup.ddx                 = node_count(lookup.lod) * info.ddx;
    lookup.ddy                 = node_count(lookup.lod) * info.ddy;
    return lookup;
}
