#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings::{mesh, config, origins, view_config, tiles, quadtree, terrain_model_approximation}
#import bevy_terrain::types::{Tile, Quadtree, NodeLookup, Blend, BestLookup, Coordinate, Morph}
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

fn compute_morph(view_distance: f32, lod: u32, grid_offset: vec2<f32>) -> Morph {
#ifdef MORPH
    let even_offset = vec2<f32>(vec2<u32>(grid_offset * view_config.grid_size) & vec2<u32>(~1u)) / view_config.grid_size;

    let target_lod  = log2(view_config.morph_distance / view_distance);
    let ratio       = select(inverse_mix(f32(lod) + view_config.morph_range, f32(lod), target_lod), 0.0, lod == 0);

    return Morph(ratio, mix(grid_offset, even_offset, ratio));
#else
    return Morph(0.0, grid_offset);
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

fn compute_grid_offset(grid_index: u32) -> vec2<f32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;
    let offset = vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);

    return vec2<f32>(offset) / view_config.grid_size;
}

 fn compute_local_position(tile: Tile, offset: vec2<f32>) -> vec3<f32> {
    let st = (vec2<f32>(tile.xy) + offset) / tile_count(tile.lod);

#ifdef SPHERICAL
    let uv = cube_to_sphere(st);

    var local_position: vec3<f32>;

    switch (tile.side) {
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

fn compute_relative_position(tile: Tile, grid_offset: vec2<f32>) -> vec3<f32> {
    let params = terrain_model_approximation.sides[tile.side];

    let lod_difference = tile.lod - u32(terrain_model_approximation.origin_lod);
    let origin_xy = vec2<i32>(params.origin_xy.x << lod_difference, params.origin_xy.y << lod_difference);
    let tile_offset = vec2<i32>(tile.xy) - origin_xy;
    let relative_st = (vec2<f32>(tile_offset) + grid_offset) / tile_count(tile.lod) + params.delta_relative_st;

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

fn approximate_view_distance(tile: Tile, offset: vec2<f32>, view_world_position: vec3<f32>) -> f32 {
    let local_position = compute_local_position(tile, offset);
    var world_position = position_local_to_world(local_position);
    let world_normal   = normal_local_to_world(local_position);
    var view_distance  = distance(world_position + terrain_model_approximation.approximate_height * world_normal, view_world_position);

#ifdef TEST1
    if (view_distance < view_config.precision_threshold_distance) {
        let relative_position = compute_relative_position(tile, offset);
        view_distance         = length(relative_position + terrain_model_approximation.approximate_height * world_normal);
    }
#endif

    return view_distance;
}

fn compute_subdivision_offset(tile: Tile) -> vec2<f32> {
    let params       = terrain_model_approximation.sides[tile.side];
    let view_tile_xy = params.view_st * tile_count(tile.lod);
    let tile_offset  = vec2<i32>(view_tile_xy) - vec2<i32>(tile.xy);
    var offset       = view_tile_xy % 1.0;

    if      (tile_offset.x < 0) { offset.x = 0.0; }
    else if (tile_offset.x > 0) { offset.x = 1.0; }
    if      (tile_offset.y < 0) { offset.y = 0.0; }
    else if (tile_offset.y > 0) { offset.y = 1.0; }

    return offset;
}

fn tile_count(lod: u32) -> f32 { return f32(1u << lod); }
fn node_count(lod: u32) -> f32 { return f32(1u << lod); }

fn inside_square(position: vec2<f32>, origin: vec2<f32>, size: f32) -> f32 {
    let inside = step(origin, position) * step(position, origin + size);

    return inside.x * inside.y;
}

// Todo: use this node as quadtree lod again
fn lookup_best(tile: Tile, offset: vec2<f32>) -> BestLookup {
    let tile_size = 1u << u32(tile.lod);

    let quadtree_side = tile.side;
    var quadtree_lod: u32;       var new_quadtree_lod = 0u;
    var node_xy:      vec2<i32>; var new_node_xy      = vec2<i32>(0);
    var node_uv:      vec2<f32>; var new_node_uv      = (vec2<f32>(tile.xy % tile_size) + offset) / f32(tile_size);
    var quadtree_uv:  vec2<f32>; var new_quadtree_uv  = new_node_uv;

    while (new_quadtree_lod < config.lod_count && !any(new_quadtree_uv < vec2<f32>(0.0)) && !any(new_quadtree_uv > vec2<f32>(1.0))) {
        quadtree_lod = new_quadtree_lod;
        quadtree_uv  = new_quadtree_uv;
        node_xy      = new_node_xy;
        node_uv      = new_node_uv;

        new_quadtree_lod += 1u;

        let lod_difference = i32(tile.lod) - i32(new_quadtree_lod);

        if (lod_difference < 0) {
            let size = 1u << u32(-lod_difference);
            let scaled_offset = offset * f32(size);
            new_node_xy = vec2<i32>(tile.xy * size) + vec2<i32>(scaled_offset);
            new_node_uv = scaled_offset % 1.0;
        } else {
            let size = 1u << u32(lod_difference);
            new_node_xy = vec2<i32>(tile.xy / size);
            new_node_uv = (vec2<f32>(tile.xy % size) + offset) / f32(size);
        }

        let origin_xy = vec2<i32>(origins[quadtree_side * config.lod_count + new_quadtree_lod]);
        let quadtree_size = f32(min(view_config.quadtree_size, 1u << new_quadtree_lod));

        new_quadtree_uv = (vec2<f32>(new_node_xy - origin_xy) + new_node_uv) / quadtree_size;
    }

    let quadtree_xy    = vec2<u32>(node_xy) % view_config.quadtree_size;
    let quadtree_index = ((quadtree_side  * config.lod_count +
                           quadtree_lod)  * view_config.quadtree_size +
                           quadtree_xy.x) * view_config.quadtree_size +
                           quadtree_xy.y;

    let quadtree_entry = quadtree[quadtree_index];

    let node_scale = i32(1u << (quadtree_lod - quadtree_entry.atlas_lod));
    let atlas_uv = (vec2<f32>(node_xy / node_scale) +
                   (vec2<f32>(node_xy % node_scale) + node_uv) / f32(node_scale)) % 1.0;

    var lookup: BestLookup;
    lookup.atlas_index  = quadtree_entry.atlas_index;
    lookup.atlas_lod    = quadtree_entry.atlas_lod;
    lookup.atlas_uv     = atlas_uv;
    lookup.quadtree_lod = quadtree_lod;
    lookup.quadtree_uv  = quadtree_uv;
    lookup.node_xy      = node_xy / node_scale;
    return lookup;
}

fn lookup_node(tile: Tile, offset: vec2<f32>, offset_dx: vec2<f32>, offset_dy: vec2<f32>, blend: Blend, lod_offset: u32) -> NodeLookup {
    let quadtree_lod   = blend.lod - lod_offset;
    let quadtree_side  = tile.side;
    let quadtree_xy    = vec2<u32>(tile.xy.x >> (tile.lod - quadtree_lod),
                                   tile.xy.y >> (tile.lod - quadtree_lod)) % view_config.quadtree_size;
    let quadtree_index = ((quadtree_side  * config.lod_count +
                           quadtree_lod)  * view_config.quadtree_size +
                           quadtree_xy.x) * view_config.quadtree_size +
                           quadtree_xy.y;
    let quadtree_entry = quadtree[quadtree_index];

    // assumes the tile lod is larger than the node lod
    // this can be guaranteed with morph distance > blend distance
    let tiles_per_node = 1u << (tile.lod - quadtree_entry.atlas_lod);

    var lookup: NodeLookup;
    lookup.lod   = quadtree_entry.atlas_lod;
    lookup.index = quadtree_entry.atlas_index;
    lookup.uv    = (vec2<f32>(tile.xy % tiles_per_node) + offset) / f32(tiles_per_node);
    lookup.ddx   = offset_dx / f32(tiles_per_node);
    lookup.ddy   = offset_dy / f32(tiles_per_node);
    return lookup;
}
