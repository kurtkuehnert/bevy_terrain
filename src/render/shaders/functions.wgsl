#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings::{config, view_config, tiles, quadtree}
#import bevy_terrain::types::{Tile, Quadtree, NodeLookup, Blend, LookupInfo, UVCoordinate}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh
#import bevy_render::maths::affine_to_square

const F0 = 0u;
const F1 = 1u;
const PS = 2u;
const PT = 3u;
const C  = 0.87 * 0.87;

fn sphere_to_cube(xy: vec2<f32>) -> vec2<f32> {
    var uv: vec2<f32>;

    // s2 quadtratic as per https://docs.s2cell.aliddell.com/en/stable/s2_concepts.html#st
    if (xy.x > 0.0) { uv.x =       0.5 * sqrt(1.0 + 3.0 * xy.x); }
    else            { uv.x = 1.0 - 0.5 * sqrt(1.0 - 3.0 * xy.x); }

    if (xy.y > 0.0) { uv.y =       0.5 * sqrt(1.0 + 3.0 * xy.y); }
    else            { uv.y = 1.0 - 0.5 * sqrt(1.0 - 3.0 * xy.y); }

    // algebraic sigmoid c = 0.87 as per https://marlam.de/publications/cubemaps/lambers2019cubemaps.pdf
    // uv = 0.5 * xy + 0.5;
    // uv *= sqrt((1 + C) / (1 + C * uv * uv));

    return uv;
}

fn cube_to_sphere(uv: vec2<f32>) -> vec2<f32> {
    var xy: vec2<f32>;

    // s2 quadtratic as per https://docs.s2cell.aliddell.com/en/stable/s2_concepts.html#st
    if (uv.x > 0.5) { xy.x =       (4.0 * pow(uv.x, 2.0) - 1.0) / 3.0; }
    else            { xy.x = (1.0 - 4.0 * pow(1.0 - uv.x, 2.0)) / 3.0; }

    if (uv.y > 0.5) { xy.y =       (4.0 * pow(uv.y, 2.0) - 1.0) / 3.0; }
    else            { xy.y = (1.0 - 4.0 * pow(1.0 - uv.y, 2.0)) / 3.0; }

    // algebraic sigmoid c = 0.87 as per https://marlam.de/publications/cubemaps/lambers2019cubemaps.pdf
    // xy = 2 * uv - 1;
    // xy /= sqrt(1 + C - C * xy * xy);

    return xy;
}

fn local_to_world_position(local_position: vec3<f32>) -> vec4<f32> {
    return affine_to_square(mesh[0].model) * vec4<f32>(local_position, 1.0);
}

fn world_to_clip_position(world_position: vec4<f32>) -> vec4<f32> {
    return view.view_proj * world_position;
}

fn compute_morph(view_distance: f32, lod: u32) -> f32 {
    let threshold_distance = 2.0 * view_config.morph_distance * tile_size(lod);
    return clamp(1.0 - (1.0 - view_distance / threshold_distance) / view_config.morph_range, 0.0, 1.0);
}

fn compute_blend(view_distance: f32) -> Blend {
    let lod_f32 = log2(2.0 * view_config.blend_distance / view_distance);
    let lod     = clamp(u32(lod_f32), 0u, config.lod_count - 1u);

#ifdef BLEND
    let ratio = select(1.0 - (lod_f32 % 1.0) / view_config.blend_range, 0.0, lod_f32 < 1.0 || lod_f32 > f32(config.lod_count));
#else
    let ratio = 0.0;
#endif

    return Blend(lod, ratio);
}

fn grid_offset(grid_index: u32) -> vec2<u32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;

    return vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);
}

fn local_position_from_coordinate(coordinate: UVCoordinate, height: f32) -> vec3<f32> {
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

    return (1.0 + height) * normalize(local_position);
#else
    return vec3<f32>(coordinate.uv.x - 0.5, height, coordinate.uv.y - 0.5);
#endif
}

fn coordinate_from_local_position(local_position: vec3<f32>) -> UVCoordinate {
#ifdef SPHERICAL
    let normal = normalize(local_position);
    let abs_normal = abs(normal);

    var side: u32; var xy: vec2<f32>;

    if (abs_normal.x > abs_normal.y && abs_normal.x > abs_normal.z) {
        if (normal.x < 0.0) { side = 0u; xy = vec2(-normal.z,  normal.y); }
        else                { side = 3u; xy = vec2(-normal.y,  normal.z); }

        xy /= normal.x;
    } else if (abs_normal.z > abs_normal.y) {
        if (normal.z > 0.0) { side = 1u; xy = vec2( normal.x, -normal.y); }
        else                { side = 4u; xy = vec2( normal.y, -normal.x); }

        xy /= normal.z;
    } else {
        if (normal.y > 0.0) { side = 2u; xy = vec2( normal.x,  normal.z); }
        else                { side = 5u; xy = vec2(-normal.z, -normal.x); }

        xy /= normal.y;
    }

    let uv = sphere_to_cube(xy);

    return UVCoordinate(side, uv);
#else
    return UVCoordinate(0u, local_position.xz + 0.5);
#endif
}

fn coordinate_project_to_side(coordinate: UVCoordinate, side: u32) -> UVCoordinate {
    var EVEN_LIST = array(
        vec2(PS, PT),
        vec2(F0, PT),
        vec2(F0, PS),
        vec2(PT, PS),
        vec2(PT, F0),
        vec2(PS, F0),
    );
    var ODD_LIST = array(
        vec2(PS, PT),
        vec2(PS, F1),
        vec2(PT, F1),
        vec2(PT, PS),
        vec2(F1, PS),
        vec2(F1, PT),
    );

    let index = (6u + side - coordinate.side) % 6u;
    let info  = select(ODD_LIST[index], EVEN_LIST[index], coordinate.side % 2u == 0u);

    var uv: vec2<f32>;

    switch info.x {
        case F0: { uv.x = 0.0; }
        case F1: { uv.x = 1.0; }
        case PS: { uv.x = coordinate.uv.x; }
        case PT: { uv.x = coordinate.uv.y; }
        default: {}
    }

    switch info.y {
        case F0: { uv.y = 0.0; }
        case F1: { uv.y = 1.0; }
        case PS: { uv.y = coordinate.uv.x; }
        case PT: { uv.y = coordinate.uv.y; }
        default: {}
    }

    return UVCoordinate(side, uv);
}

fn tile_size(lod: u32) -> f32 {
    return 1.0 / f32(1u << lod);
}

fn tile_coordinate(tile: Tile, offset: vec2<f32>) -> UVCoordinate {
    return UVCoordinate(tile.side, (vec2<f32>(tile.xy) + offset) * tile_size(tile.lod));
}

fn node_count(lod: u32) -> f32 {
    return f32(1u << lod);
}

fn node_coordinate(coordinate: UVCoordinate, lod: u32) -> vec2<f32> {
    return min(coordinate.uv, vec2(0.9999999)) * node_count(lod);
}

fn inside_square(position: vec2<f32>, origin: vec2<f32>, size: f32) -> f32 {
    let inside = step(origin, position) * step(position, origin + size);

    return inside.x * inside.y;
}

fn quadtree_origin(quadtree_coordinate: UVCoordinate, lod: u32) -> vec2<f32> {
    let node_coordinate = node_coordinate(quadtree_coordinate, lod);
    let max_offset      = node_count(lod) - f32(view_config.quadtree_size);

    return clamp(round(node_coordinate - 0.5 * f32(view_config.quadtree_size)), vec2<f32>(0.0), vec2<f32>(max_offset));
}

fn inside_quadtree(view_coordinate: UVCoordinate, frag_coordinate: UVCoordinate, lod: u32) -> f32 {
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

fn quadtree_lod(frag_coordinate: UVCoordinate) -> u32 {
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
    let quadtree_side       = info.coordinate.side;
    let quadtree_coordinate = vec2<u32>(node_coordinate(info.coordinate, quadtree_lod)) % view_config.quadtree_size;
    let quadtree_index      = ((quadtree_side          * config.lod_count +
                                quadtree_lod)          * view_config.quadtree_size +
                                quadtree_coordinate.x) * view_config.quadtree_size +
                                quadtree_coordinate.y;

    let entry               = quadtree.data[quadtree_index];

    let atlas_lod           = entry.atlas_lod;
    let atlas_index         = entry.atlas_index;
    let atlas_coordinate    = node_coordinate(info.coordinate, atlas_lod) % 1.0;
    let atlas_ddx           = node_count(atlas_lod) * info.ddx;
    let atlas_ddy           = node_count(atlas_lod) * info.ddy;

    return NodeLookup(atlas_index, atlas_lod, atlas_coordinate, atlas_ddx, atlas_ddy);
}
