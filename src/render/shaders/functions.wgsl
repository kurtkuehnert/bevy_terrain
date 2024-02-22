#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings::{config, view_config, tiles, quadtree}
#import bevy_terrain::types::{Tile, Quadtree, NodeLookup, Morph, Blend, LookupInfo, S2Coordinate}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh
#import bevy_render::maths::affine_to_square

fn local_to_world_position(local_position: vec3<f32>) -> vec4<f32> {
    return affine_to_square(mesh[0].model) * vec4<f32>(local_position, 1.0);
}

fn world_to_clip_position(world_position: vec4<f32>) -> vec4<f32> {
    return view.view_proj * world_position;
}

fn compute_morph(coordinate: S2Coordinate, lod: u32) -> Morph {
    let local_position = local_position_from_coordinate(coordinate, view_config.approximate_height);
    let view_distance = distance(local_position, view_config.view_local_position);
    let threshold_distance = 2.0 * view_config.morph_distance * tile_size(lod);
    let ratio = clamp(1.0 - (1.0 - view_distance / threshold_distance) / view_config.morph_range, 0.0, 1.0);

    return Morph(ratio);
}

fn compute_blend(view_distance: f32) -> Blend {
    let lod_f32 = log2(2.0 * view_config.blend_distance / view_distance);
    let lod = clamp(u32(lod_f32), 0u, config.lod_count - 1u);
    let ratio = select(1.0 - (lod_f32 % 1.0) / view_config.blend_range, 0.0, lod_f32 < 1.0 || lod_f32 > f32(config.lod_count));

    return Blend(lod, ratio);
}

fn grid_offset(grid_index: u32) -> vec2<u32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;

    return vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);
}

fn vertex_coordinate(vertex_index: u32) -> S2Coordinate {
    let tile_index = vertex_index / view_config.vertices_per_tile;
    let grid_index = vertex_index % view_config.vertices_per_tile;

    let tile        = tiles.data[tile_index];
    let grid_offset = grid_offset(grid_index);
    var coordinate  = tile_coordinate(tile, vec2<f32>(grid_offset) / view_config.grid_size);

#ifdef MESH_MORPH
    let morph        = compute_morph(coordinate, tile.lod);
    let morph_offset = mix(vec2<f32>(grid_offset), vec2<f32>(grid_offset & vec2<u32>(4294967294u)), morph.ratio);
    coordinate       = tile_coordinate(tile, morph_offset / view_config.grid_size);
#endif

    return coordinate;
}

fn local_position_from_coordinate(coordinate: S2Coordinate, height: f32) -> vec3<f32> {
#ifdef SPHERICAL
    var ORIGIN_ARRAY = array<vec3<f32>, 6u>(vec3<f32>(-1.0,  0.0,  0.0),
                                            vec3<f32>( 0.0,  0.0,  1.0),
                                            vec3<f32>( 0.0,  1.0,  0.0),
                                            vec3<f32>( 1.0,  0.0,  0.0),
                                            vec3<f32>( 0.0,  0.0, -1.0),
                                            vec3<f32>( 0.0, -1.0,  0.0));
    var U_ARRAY      = array<vec3<f32>, 6u>(vec3<f32>( 0.0,  0.0,  1.0),
                                            vec3<f32>( 1.0,  0.0,  0.0),
                                            vec3<f32>( 1.0,  0.0,  0.0),
                                            vec3<f32>( 0.0, -1.0,  0.0),
                                            vec3<f32>( 0.0, -1.0,  0.0),
                                            vec3<f32>( 0.0,  0.0,  1.0));
    var V_ARRAY      = array<vec3<f32>, 6u>(vec3<f32>( 0.0, -1.0,  0.0),
                                            vec3<f32>( 0.0, -1.0,  0.0),
                                            vec3<f32>( 0.0,  0.0,  1.0),
                                            vec3<f32>( 0.0,  0.0,  1.0),
                                            vec3<f32>( 1.0,  0.0,  0.0),
                                            vec3<f32>( 1.0,  0.0,  0.0));

    var u: f32; var v: f32;

    if (coordinate.st.x > 0.5) { u =       (4.0 * pow(coordinate.st.x, 2.0) - 1.0) / 3.0; }
    else                       { u = (1.0 - 4.0 * pow(1.0 - coordinate.st.x, 2.0)) / 3.0; }

    if (coordinate.st.y > 0.5) { v =       (4.0 * pow(coordinate.st.y, 2.0) - 1.0) / 3.0; }
    else                       { v = (1.0 - 4.0 * pow(1.0 - coordinate.st.y, 2.0)) / 3.0; }

    // switch (coordinate.side) {
    //     case 7: { u = 1; }
    // }

    // u = 2.0 * coordinate.st.x - 1.0;
    // v = 2.0 * coordinate.st.y - 1.0;
    // return ORIGIN_ARRAY[coordinate.side] + U_ARRAY[coordinate.side] * u + V_ARRAY[coordinate.side] * v;

    return (1.0 + height) * normalize(ORIGIN_ARRAY[coordinate.side] +
                                           U_ARRAY[coordinate.side] * u +
                                           V_ARRAY[coordinate.side] * v);

#else
    let uv = 2.0 * coordinate.st - 1.0;

    return vec3<f32>(uv.x, height, uv.y);
#endif
}

fn coordinate_from_local_position(local_position: vec3<f32>) -> S2Coordinate {
#ifdef SPHERICAL
    let normal = normalize(local_position);
    let abs_normal = abs(normal);

    var side: u32;
    var uv: vec2<f32>;

    if (abs_normal.x > abs_normal.y && abs_normal.x > abs_normal.z) {
        if (normal.x < 0.0) {
            side = 0u;
            uv = vec2<f32>(-normal.z / normal.x, normal.y / normal.x);
        }
        else {
            side = 3u;
            uv = vec2<f32>(-normal.y / normal.x, normal.z / normal.x);
        }
    }
    else if (abs_normal.z > abs_normal.y) {
        if (normal.z > 0.0) {
            side = 1u;
            uv = vec2<f32>(normal.x / normal.z, -normal.y / normal.z);
        }
        else {
            side = 4u;
            uv = vec2<f32>(normal.y / normal.z, -normal.x / normal.z);
        }
    }
    else {
        if (normal.y > 0.0) {
            side = 2u;
            uv = vec2<f32>(normal.x / normal.y, normal.z / normal.y);
        }
        else {
            side = 5u;
            uv = vec2<f32>(-normal.z / normal.y, -normal.x / normal.y);
        }
    }

    var st = vec2<f32>(0.0);

    if (uv.x > 0.0) { st.x =       0.5 * sqrt(1.0 + 3.0 * uv.x); }
    else            { st.x = 1.0 - 0.5 * sqrt(1.0 - 3.0 * uv.x); }

    if (uv.y > 0.0) { st.y =       0.5 * sqrt(1.0 + 3.0 * uv.y); }
    else            { st.y = 1.0 - 0.5 * sqrt(1.0 - 3.0 * uv.y); }

    return S2Coordinate(side, st);
#else
    let st = 0.5 * local_position.xz + 0.5;

    return S2Coordinate(0u, st);
#endif
}

fn coordinate_project_to_side(coordinate: S2Coordinate, side: u32) -> S2Coordinate {
    let F0 = 0u;
    let F1 = 1u;
    let PS = 2u;
    let PT = 3u;

    var EVEN_LIST = array<vec2<u32>, 6u>(
        vec2<u32>(PS, PT),
        vec2<u32>(F0, PT),
        vec2<u32>(F0, PS),
        vec2<u32>(PT, PS),
        vec2<u32>(PT, F0),
        vec2<u32>(PS, F0),
    );
    var ODD_LIST = array<vec2<u32>, 6u>(
        vec2<u32>(PS, PT),
        vec2<u32>(PS, F1),
        vec2<u32>(PT, F1),
        vec2<u32>(PT, PS),
        vec2<u32>(F1, PS),
        vec2<u32>(F1, PT),
    );

    let index = (6u + side - coordinate.side) % 6u;
    let info: vec2<u32> = select(ODD_LIST[index], EVEN_LIST[index], coordinate.side % 2u == 0u);

    var st: vec2<f32>;

    if (info.x == F0)      { st.x = 0.0; }
    else if (info.x == F1) { st.x = 1.0; }
    else if (info.x == PS) { st.x = coordinate.st.x; }
    else if (info.x == PT) { st.x = coordinate.st.y; }

    if (info.y == F0)      { st.y = 0.0; }
    else if (info.y == F1) { st.y = 1.0; }
    else if (info.y == PS) { st.y = coordinate.st.x; }
    else if (info.y == PT) { st.y = coordinate.st.y; }

    return S2Coordinate(side, st);
}

fn tile_size(lod: u32) -> f32 {
    return 1.0 / f32(1u << lod);
}

fn tile_coordinate(tile: Tile, offset: vec2<f32>) -> S2Coordinate {
     return S2Coordinate(tile.side, (vec2<f32>(tile.xy) + offset) * tile_size(tile.lod));
}

fn node_count(lod: u32) -> u32 {
    return 1u << lod;
}

fn node_coordinate(coordinate: S2Coordinate, lod: u32) -> vec2<f32> {
    let node_count = f32(node_count(lod));
    let max_coordinate  = vec2<f32>(node_count - 0.00001);

    return clamp(coordinate.st * node_count, vec2<f32>(0.0), max_coordinate);
}

fn inside_square(position: vec2<f32>, origin: vec2<f32>, size: f32) -> f32 {
    let inside = step(origin, position) * step(position, origin + size);

    return inside.x * inside.y;
}

fn quadtree_origin(quadtree_coordinate: S2Coordinate, lod: u32) -> vec2<f32> {
    let node_coordinate = node_coordinate(quadtree_coordinate, lod);
    let max_offset          = f32(node_count(lod)) - f32(view_config.quadtree_size);

    return clamp(round(node_coordinate - 0.5 * f32(view_config.quadtree_size)), vec2<f32>(0.0), vec2<f32>(max_offset));
}

fn inside_quadtree(view_coordinate: S2Coordinate, frag_coordinate: S2Coordinate, lod: u32) -> f32 {
#ifdef SPHERICAL
    let quadtree_coordinate = coordinate_project_to_side(view_coordinate, frag_coordinate.side);
#else
    let quadtree_coordinate = view_coordinate;
#endif

    let node_coordinate = floor(node_coordinate(frag_coordinate,     lod));
    let quadtree_origin = floor(quadtree_origin(quadtree_coordinate, lod));

    let node_distance = node_coordinate - quadtree_origin;

    return inside_square(node_distance, vec2<f32>(0.0), f32(view_config.quadtree_size - 1u));
}

fn quadtree_lod(frag_coordinate: S2Coordinate) -> u32 {
    let view_coordinate = coordinate_from_local_position(view_config.view_local_position);

    for (var lod = config.lod_count - 1u; lod > 0u; lod = lod - 1u) {
        if (inside_quadtree(view_coordinate, frag_coordinate, lod) == 1.0) {
            return lod;
        }
    }

    return 0u;
}

fn lookup_node(info: LookupInfo, lod_offset: u32) -> NodeLookup {
    let quadtree_lod        = info.lod - lod_offset;
    var node_coordinate     = node_coordinate(info.coordinate, quadtree_lod);
    let quadtree_side       = info.coordinate.side;
    let quadtree_coordinate = vec2<u32>(node_coordinate) % view_config.quadtree_size;
    let quadtree_index      = ((quadtree_side            * config.lod_count +
                                quadtree_lod)            * view_config.quadtree_size +
                                quadtree_coordinate.x)   * view_config.quadtree_size +
                                quadtree_coordinate.y;

    let entry               = quadtree.data[quadtree_index];
    let node_count          = f32(node_count(entry.atlas_lod));
    node_coordinate        /= f32(1u << (quadtree_lod - entry.atlas_lod));

    let atlas_lod           = entry.atlas_lod;
    let atlas_index         = entry.atlas_index;
    let atlas_coordinate    = node_coordinate % 1.0;
    let atlas_ddx           = info.ddx * node_count;
    let atlas_ddy           = info.ddy * node_count;

    return NodeLookup(atlas_index, atlas_lod, atlas_coordinate, atlas_ddx, atlas_ddy, quadtree_side);
}
