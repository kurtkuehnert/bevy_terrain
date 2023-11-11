#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings config, view_config, tiles, quadtree
#import bevy_terrain::types Tile, NodeLookup, Morph, Blend, S2Coordinate
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::mesh_bindings mesh

fn morph_threshold_distance(tile: Tile) -> f32 {
    let tile_count = 1.0 / tile.size;

    return view_config.morph_distance / tile_count;
}

fn blend_threshold_distance() -> f32 {
    return view_config.blend_distance / config.leaf_node_count;
}

fn compute_morph(tile: Tile, local_position: vec3<f32>) -> Morph {
    let view_local_position = world_to_local_position(view.world_position);

    let view_distance = distance(local_position, view_local_position);
    let threshold_distance = 2.0 * morph_threshold_distance(tile);
    let ratio = clamp(1.0 - (1.0 - view_distance / threshold_distance) / view_config.morph_range, 0.0, 1.0);

    return Morph(ratio);
}

fn compute_blend(local_position: vec3<f32>) -> Blend {
    let view_local_position = world_to_local_position(view.world_position);

    let view_distance = distance(local_position, view_local_position);
    let log_distance = max(log2(view_distance / blend_threshold_distance()), 0.0);
    let ratio = 1.0 - (1.0 - log_distance % 1.0) / view_config.blend_range;

    return Blend(u32(log_distance), ratio);
}

fn grid_offset(grid_index: u32) -> vec2<u32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;

    return vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);
}

fn tile_local_position(tile: Tile, uv_offset: vec2<f32>) -> vec3<f32> {
#ifdef SPHERICAL
    var COORDINATE_ARRAY = array<vec3<f32>, 6u>(vec3<f32>(-1.0,  1.0, -1.0),
                                                vec3<f32>(-1.0, -1.0, -1.0),
                                                vec3<f32>(-1.0, -1.0, -1.0),
                                                vec3<f32>( 1.0, -1.0, -1.0),
                                                vec3<f32>(-1.0, -1.0, -1.0),
                                                vec3<f32>(-1.0, -1.0,  1.0));
    var U_ARRAY          = array<vec3<f32>, 6u>(vec3<f32>( 2.0,  0.0,  0.0),
                                                vec3<f32>( 0.0,  0.0,  2.0),
                                                vec3<f32>( 0.0,  2.0,  0.0),
                                                vec3<f32>( 0.0,  0.0,  2.0),
                                                vec3<f32>( 2.0,  0.0,  0.0),
                                                vec3<f32>( 0.0,  2.0,  0.0));
    var V_ARRAY          = array<vec3<f32>, 6u>(vec3<f32>( 0.0,  0.0,  2.0),
                                                vec3<f32>( 2.0,  0.0,  0.0),
                                                vec3<f32>( 0.0,  0.0,  2.0),
                                                vec3<f32>( 0.0,  2.0,  0.0),
                                                vec3<f32>( 0.0,  2.0,  0.0),
                                                vec3<f32>( 2.0,  0.0,  0.0));
#else
    var COORDINATE_ARRAY = array<vec3<f32>, 1u>(vec3<f32>(-1.0,  0.0, -1.0));
    var U_ARRAY          = array<vec3<f32>, 1u>(vec3<f32>( 2.0,  0.0,  0.0));
    var V_ARRAY          = array<vec3<f32>, 1u>(vec3<f32>( 0.0,  0.0,  2.0));
#endif

    let uv = tile.uv + tile.size * uv_offset;
    var local_position = COORDINATE_ARRAY[tile.side] + uv.x * U_ARRAY[tile.side] + uv.y * V_ARRAY[tile.side];

#ifdef SPHERICAL
    let p = local_position;
    let p2 = p * p;

    local_position = vec3<f32>(p.x * sqrt(1.0 - (p2.y + p2.z) / 2.0 + p2.y * p2.z / 3.0),
                               p.y * sqrt(1.0 - (p2.x + p2.z) / 2.0 + p2.x * p2.z / 3.0),
                               p.z * sqrt(1.0 - (p2.x + p2.y) / 2.0 + p2.x * p2.y / 3.0));
#endif

    return local_position;
}

fn vertex_local_position(vertex_index: u32) -> vec3<f32> {
    let tile_index = vertex_index / view_config.vertices_per_tile;
    let grid_index = vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];

    let grid_offset = grid_offset(grid_index);
    let grid_uv = vec2<f32>(grid_offset) / view_config.grid_size;
    var local_position = tile_local_position(tile, grid_uv);

#ifdef MESH_MORPH
    let morph = compute_morph(tile, local_position);

    let even_grid_offset = grid_offset & vec2<u32>(4294967294u); // set last bit to zero
    let even_grid_uv = vec2<f32>(even_grid_offset) / view_config.grid_size;
    let even_local_position = tile_local_position(tile, even_grid_uv);

    local_position = mix(local_position, even_local_position, morph.ratio);
#endif

    return local_position;
}

fn local_position_apply_height(local_position: vec3<f32>, height: f32) -> vec3<f32> {
#ifdef SPHERICAL
    return local_position * (1.0 + height);
#else
    return local_position + vec3<f32>(0.0, height, 0.0);
#endif
}

fn local_to_world_position(local_position: vec3<f32>) -> vec4<f32> {
    return mesh.model * vec4<f32>(local_position, 1.0);
}

fn world_to_local_position(world_position: vec3<f32>) -> vec3<f32> {
    return (mesh.inverse_transpose_model * vec4<f32>(world_position, 1.0)).xyz;
}

// https://docs.s2cell.aliddell.com/en/stable/s2_concepts.html#lat-lon-to-s2-cell-id
// uses adjusted logic to match bevys coordinate system
fn s2_from_local_position(local_position: vec3<f32>) -> S2Coordinate {
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

fn s2_to_local_position(s2: S2Coordinate) -> vec3<f32> {
#ifdef SPHERICAL
    var uv = vec2<f32>(0.0);

    if (s2.st.x > 0.5) { uv.x =       (4.0 * pow(s2.st.x, 2.0) - 1.0) / 3.0; }
    else               { uv.x = (1.0 - 4.0 * pow(1.0 - s2.st.x, 2.0)) / 3.0; }

    if (s2.st.y > 0.5) { uv.y =       (4.0 * pow(s2.st.y, 2.0) - 1.0) / 3.0; }
    else               { uv.y = (1.0 - 4.0 * pow(1.0 - s2.st.y, 2.0)) / 3.0; }

    var local_position: vec3<f32>;

    if (s2.side == 0u) {
        local_position = vec3<f32>(-1.0, -uv.y, uv.x);
    }
    else if (s2.side == 1u) {
        local_position = vec3<f32>(uv.x, -uv.y, 1.0);
    }
    else if (s2.side == 2u) {
        local_position = vec3<f32>(uv.x, 1.0, uv.y);
    }
    else if (s2.side == 3u) {
        local_position = vec3<f32>(1.0, -uv.x, uv.y);
    }
    else if (s2.side == 4u) {
        local_position = vec3<f32>(uv.y, -uv.x, -1.0);
    }
    else if (s2.side == 5u) {
        local_position = vec3<f32>(uv.y, -1.0, uv.x);
    }

    return normalize(local_position);
#else
    return vec3<f32>(2.0 * s2.st.x - 1.0, 0.0, 2.0 * s2.st.y - 1.0);
#endif
}

fn s2_project_to_side(s2: S2Coordinate, side: u32) -> S2Coordinate {
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

    let index = (6u + side - s2.side) % 6u;

    var info: vec2<u32>;
    var st: vec2<f32>;

    if (s2.side % 2u == 0u) { info = EVEN_LIST[index]; }
    else                    { info =  ODD_LIST[index]; }

    if (info.x == F0)      { st.x = 0.0; }
    else if (info.x == F1) { st.x = 1.0; }
    else if (info.x == PS) { st.x = s2.st.x; }
    else if (info.x == PT) { st.x = s2.st.y; }

    if (info.y == F0)      { st.y = 0.0; }
    else if (info.y == F1) { st.y = 1.0; }
    else if (info.y == PS) { st.y = s2.st.x; }
    else if (info.y == PT) { st.y = s2.st.y; }

    return S2Coordinate(side, st);
}

fn node_count(lod: u32) -> f32 {
    return config.leaf_node_count / f32(1u << lod);
}

fn node_coordinate(st: vec2<f32>, lod: u32) -> vec2<f32> {
    return st * node_count(lod);
}

fn inside_square(position: vec2<f32>, origin: vec2<f32>, size: f32) -> f32 {
    let inside = step(origin, position) * step(position, origin + size);

    return inside.x * inside.y;
}

fn inside_quadtree(view_s2: S2Coordinate, frag_s2: S2Coordinate, lod: u32) -> f32 {
#ifdef SPHERICAL
    let quadtree_s2 = s2_project_to_side(view_s2, frag_s2.side);
#else
    let quadtree_s2 = view_s2;
#endif

    let frag_coordinate            = node_coordinate(frag_s2.st,     lod);
    let quadtree_coordinate        = node_coordinate(quadtree_s2.st, lod);
    let max_offset                 = ceil(node_count(lod)) - f32(view_config.quadtree_size);
    let quadtree_origin_coordinate = clamp(round(quadtree_coordinate - 0.5 * f32(view_config.quadtree_size)), vec2<f32>(0.0), vec2<f32>(max_offset));

    let dist = floor(frag_coordinate) - floor(quadtree_origin_coordinate);

    return inside_square(dist, vec2<f32>(0.0), f32(view_config.quadtree_size - 1u));
}

fn quadtree_lod(frag_local_position: vec3<f32>) -> u32 {
    let view_local_position = world_to_local_position(view.world_position); // Todo: adjust for new world_to_local funktion
    let view_s2 = s2_from_local_position(view_local_position);
    let frag_s2 = s2_from_local_position(frag_local_position);

    var lod = 0u;

    loop {
        if (inside_quadtree(view_s2, frag_s2, lod) == 1.0 || lod == config.lod_count - 1u) { break; }

        lod = lod + 1u;
    }

    return lod;
}

fn lookup_node(local_position: vec3<f32>, lod: u32) -> NodeLookup {
#ifdef QUADTREE_LOD
    let lod = quadtree_lod(local_position);
#endif

    let s2 = s2_from_local_position(local_position);

    let quadtree_lod        = min(lod, config.lod_count - 1u);
    let quadtree_index      = s2.side * config.lod_count + quadtree_lod;
    let quadtree_coordinate = vec2<u32>(node_coordinate(s2.st, quadtree_lod)) % view_config.quadtree_size;

    let lookup = textureLoad(quadtree, quadtree_coordinate, quadtree_index, 0);

    let atlas_lod        = lookup.y;
    let atlas_index      = lookup.x;
    let atlas_coordinate = node_coordinate(s2.st, atlas_lod) % 1.0;

    return NodeLookup(atlas_index, atlas_lod, atlas_coordinate);
}
