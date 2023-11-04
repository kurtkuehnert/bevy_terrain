#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings config, view_config, quadtree, atlas_sampler
#import bevy_terrain::types Tile, NodeLookup, Blend, S2Coordinate
#import bevy_terrain::attachments height_atlas, HEIGHT_SIZE,
#import bevy_pbr::mesh_view_bindings view

fn calculate_plane_position(coordinate: vec3<f32>) -> vec3<f32> {
    let p = coordinate - 0.5; // [-0.5, 0.5]

    return p * config.terrain_size;
}

fn calculate_sphere_position(coordinate: vec3<f32>) -> vec3<f32> {
    let p = 2.0 * coordinate - 1.0;
    let x2 = p.x * p.x;
	let y2 = p.y * p.y;
	let z2 = p.z * p.z;

	let rx = p.x * sqrt(1.0 - (y2 + z2) / 2.0 + y2 * z2 / 3.0);
	let ry = p.y * sqrt(1.0 - (x2 + z2) / 2.0 + x2 * z2 / 3.0);
	let rz = p.z * sqrt(1.0 - (x2 + y2) / 2.0 + x2 * y2 / 3.0);

    let r = vec3<f32>(rx, ry, rz);

    return r * config.radius;
}

fn approximate_world_position(local_position: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(local_position, 1.0);
}

fn tile_coordinate(tile: Tile, uv_offset: vec2<f32>) -> vec3<f32> {
#ifdef SPHERICAL
    var COORDINATE_ARRAY = array<vec3<f32>, 6u>(
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );

    var U_ARRAY = array<vec3<f32>, 6u>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
    );

    var V_ARRAY = array<vec3<f32>, 6u>(
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(1.0, 0.0, 0.0),
    );

    let coordinate = COORDINATE_ARRAY[tile.side];
    let u          = U_ARRAY[tile.side];
    let v          = V_ARRAY[tile.side];
#else
    let coordinate = vec3<f32>(0.0, 0.5, 0.0);
    let u          = vec3<f32>(1.0, 0.0, 0.0);
    let v          = vec3<f32>(0.0, 0.0, 1.0);
#endif

    let uv = tile.uv + tile.size * uv_offset;

    return coordinate + uv.x * u + uv.y * v;
}

fn tile_local_position(tile: Tile, uv_offset: vec2<f32>) -> vec3<f32> {
    let coordinate = tile_coordinate(tile, uv_offset);

#ifdef SPHERICAL
    let local_position = calculate_sphere_position(coordinate);
#else
    let local_position = calculate_plane_position(coordinate);
#endif

    return local_position;
}

fn morph_threshold_distance(tile: Tile) -> f32 {
#ifdef SPHERICAL
    return tile.size * config.radius * view_config.morph_distance;
#else
    return tile.size * config.terrain_size * view_config.morph_distance;
#endif
}

fn morph(tile: Tile, world_position: vec4<f32>) -> f32 {
    let viewer_distance = distance(world_position.xyz, view.world_position.xyz);
    let threshold_distance = 2.0 * morph_threshold_distance(tile);
    let ratio = clamp(1.0 - (1.0 - viewer_distance / threshold_distance) / view_config.morph_range, 0.0, 1.0);

    return ratio;
}

fn blend(world_position: vec4<f32>) -> Blend {
    let viewer_distance = distance(world_position.xyz, view.world_position.xyz);
    let log_distance = max(log2(viewer_distance / view_config.blend_distance), 0.0);
    let ratio = (1.0 - log_distance % 1.0) / view_config.blend_range;

    return Blend(u32(log_distance), ratio);
}

fn grid_offset(grid_index: u32) -> vec2<u32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;

    return vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);
}

fn vertex_local_position(tile: Tile, grid_index: u32) -> vec3<f32> {
    let grid_offset = grid_offset(grid_index);
    let grid_uv = vec2<f32>(grid_offset) / view_config.grid_size;
    var local_position = tile_local_position(tile, grid_uv);

    #ifdef MESH_MORPH
        let world_position = approximate_world_position(local_position);
        let morph = morph(tile, world_position);

        let even_grid_offset = grid_offset & vec2<u32>(4294967294u); // set last bit to zero
        let even_grid_uv = vec2<f32>(even_grid_offset) / view_config.grid_size;
        let even_local_position = tile_local_position(tile, even_grid_uv);

        local_position = mix(local_position, even_local_position, morph);
    #endif

    return local_position;
}

// https://docs.s2cell.aliddell.com/en/stable/s2_concepts.html#lat-lon-to-s2-cell-id
// uses adjusted logic to match bevys coordinate system
fn s2_from_world_position(world_position: vec4<f32>) -> S2Coordinate {
#ifdef SPHERICAL
    let local_position = world_position.xyz;

    let direction = normalize(local_position);
    let abs_direction = abs(direction);

    var side: u32;
    var uv: vec2<f32>;

    if (abs_direction.x > abs_direction.y && abs_direction.x > abs_direction.z) {
        if (direction.x < 0.0) {
            side = 0u;
            uv = vec2<f32>(-direction.z / direction.x, direction.y / direction.x);
        }
        else {
            side = 3u;
            uv = vec2<f32>(-direction.y / direction.x, direction.z / direction.x);
        }
    }
    else if (abs_direction.z > abs_direction.y) {
        if (direction.z > 0.0) {
            side = 1u;
            uv = vec2<f32>(direction.x / direction.z, -direction.y / direction.z);
        }
        else {
            side = 4u;
            uv = vec2<f32>(direction.y / direction.z, -direction.x / direction.z);
        }
    }
    else {
        if (direction.y > 0.0) {
            side = 2u;
            uv = vec2<f32>(direction.x / direction.y, direction.z / direction.y);
        }
        else {
            side = 5u;
            uv = vec2<f32>(-direction.z / direction.y, -direction.x / direction.y);
        }
    }

    var st = vec2<f32>(0.0);

    if (uv.x > 0.0) { st.x =       0.5 * sqrt(1.0 + 3.0 * uv.x); }
    else            { st.x = 1.0 - 0.5 * sqrt(1.0 - 3.0 * uv.x); }

    if (uv.y > 0.0) { st.y =       0.5 * sqrt(1.0 + 3.0 * uv.y); }
    else            { st.y = 1.0 - 0.5 * sqrt(1.0 - 3.0 * uv.y); }

    return S2Coordinate(side, st);
#else
    let local_position = world_position.xyz;

    let st = local_position.xz / config.terrain_size + 0.5;

    return S2Coordinate(0u, st);
#endif
}

fn s2_to_world_position(s2: S2Coordinate) -> vec4<f32> {
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

    local_position = normalize(local_position);

    let world_position = vec4<f32>(local_position * config.radius, 1.0);

    return world_position;
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
    else                         { info =  ODD_LIST[index]; }

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

fn nodes_per_side(lod: u32) -> f32 {
    return config.nodes_per_side / f32(1u << lod);
}

fn node_coordinate(st: vec2<f32>, lod: u32) -> vec2<f32> {
    return st * nodes_per_side(lod);
}

fn inside_rect(position: vec2<f32>, origin: vec2<f32>, size: f32) -> f32 {
    let inside = step(origin, position) * step(position, origin + size);

    return inside.x * inside.y;
}

fn inside_quadtree(view_s2: S2Coordinate, frag_s2: S2Coordinate, lod: u32) -> f32 {
    let frag_coordinate = node_coordinate(frag_s2.st, lod);

    let quadtree_s2 = s2_project_to_side(view_s2, frag_s2.side);
    let quadtree_coordinate = node_coordinate(quadtree_s2.st, lod);
    let max_offset = ceil(nodes_per_side(lod)) - f32(view_config.node_count);
    let quadtree_origin_coordinate = clamp(round(quadtree_coordinate - 0.5 * f32(view_config.node_count)), vec2<f32>(0.0), vec2<f32>(max_offset));

    let dist = floor(frag_coordinate) - floor(quadtree_origin_coordinate);

    return inside_rect(dist, vec2<f32>(0.0), f32(view_config.node_count - 1u));
}

fn quadtree_lod(world_position: vec4<f32>) -> u32 {
    let view_s2 = s2_from_world_position(vec4<f32>(view.world_position, 1.0));
    let frag_s2 = s2_from_world_position(world_position);

    var lod = 0u;

    loop {
        if (inside_quadtree(view_s2, frag_s2, lod) == 1.0 || lod == config.lod_count - 1u) { break; }

        lod = lod + 1u;
    }

    return lod;
}

fn lookup_node(world_position: vec4<f32>, lod: u32) -> NodeLookup {
#ifdef QUADTREE_LOD
    let lod = quadtree_lod(world_position);
#endif

    let s2 = s2_from_world_position(world_position);

    let quadtree_lod        = min(lod, config.lod_count - 1u);
    let quadtree_index      = s2.side * config.lod_count + quadtree_lod;
    let quadtree_coordinate = vec2<u32>(node_coordinate(s2.st, quadtree_lod)) % view_config.node_count;

    let lookup = textureLoad(quadtree, quadtree_coordinate, quadtree_index, 0);

    let atlas_lod        = lookup.y;
    let atlas_index      = lookup.x;
    let atlas_coordinate = node_coordinate(s2.st, atlas_lod) % 1.0;

    return NodeLookup(atlas_index, atlas_lod, atlas_coordinate);
}

fn calculate_normal(coords: vec2<f32>, atlas_index: i32, atlas_lod: u32, ddx: vec2<f32>, ddy: vec2<f32>) -> vec3<f32> {
#ifdef SAMPLE_GRAD
    let offset = 1.0 / HEIGHT_SIZE;
    let left  = textureSampleGrad(height_atlas, atlas_sampler, coords + vec2<f32>(-offset,     0.0), atlas_index, ddx, ddy).x;
    let up    = textureSampleGrad(height_atlas, atlas_sampler, coords + vec2<f32>(    0.0, -offset), atlas_index, ddx, ddy).x;
    let right = textureSampleGrad(height_atlas, atlas_sampler, coords + vec2<f32>( offset,     0.0), atlas_index, ddx, ddy).x;
    let down  = textureSampleGrad(height_atlas, atlas_sampler, coords + vec2<f32>(    0.0,  offset), atlas_index, ddx, ddy).x;
#else
    let left  = textureSampleLevel(height_atlas, atlas_sampler, coords, atlas_index, 0.0, vec2<i32>(-1,  0)).x;
    let up    = textureSampleLevel(height_atlas, atlas_sampler, coords, atlas_index, 0.0, vec2<i32>( 0, -1)).x;
    let right = textureSampleLevel(height_atlas, atlas_sampler, coords, atlas_index, 0.0, vec2<i32>( 1,  0)).x;
    let down  = textureSampleLevel(height_atlas, atlas_sampler, coords, atlas_index, 0.0, vec2<i32>( 0,  1)).x;

#endif

    return normalize(vec3<f32>(right - left, f32(2u << atlas_lod) / config.height, down - up));
}
