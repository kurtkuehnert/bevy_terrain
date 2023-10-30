#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings config, view_config, quadtree, atlas_sampler
#import bevy_terrain::types Tile, NodeLookup, Blend, S2Coordinate
#import bevy_terrain::attachments height_atlas, HEIGHT_SIZE,
#import bevy_pbr::mesh_view_bindings view

fn calculate_plane_position(coordinate: vec3<f32>) -> vec3<f32> {
    let p = coordinate - 0.5;

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

    // let r = p;
    // let r = normalize(p);
    let r = vec3<f32>(rx, ry, rz);

    return r * config.radius;
}

fn approximate_world_position(local_position: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(local_position, 1.0);
}

fn tile_coordinate(tile: Tile, uv_offset: vec2<f32>) -> vec3<f32> {
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

    let uv = tile.uv + tile.size * uv_offset;

    return COORDINATE_ARRAY[tile.side] + uv.x * U_ARRAY[tile.side] + uv.y * V_ARRAY[tile.side];
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
    return tile.size * config.radius * view_config.view_distance;
#else
    return tile.size * config.terrain_size * view_config.view_distance;
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
    let threshold_distance = 2.0 * view_config.view_distance;
    let log_distance = max(log2(viewer_distance / threshold_distance), 0.0);
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
        let world_position = vec4<f32>(local_position, 1.0);
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
fn world_position_to_s2_coordinate(world_position: vec4<f32>) -> S2Coordinate {
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
}

fn nodes_per_side(lod: u32) -> f32 {
    return config.nodes_per_side / f32(1u << lod);
}

fn lookup_node(lod: u32, world_position: vec4<f32>) -> NodeLookup {
    let s2_coordinate = world_position_to_s2_coordinate(world_position);
    let st = s2_coordinate.st;
    let side = s2_coordinate.side;

    let quadtree_lod = min(lod, config.lod_count - 1u);
    let nodes_per_side = nodes_per_side(quadtree_lod);
    let node_coordinate = st * nodes_per_side; // Todo: replace with fract(node_coordinate)
    let quadtree_coordinate = vec2<i32>(node_coordinate) % i32(view_config.node_count);

    let lookup = textureLoad(quadtree, quadtree_coordinate, side * config.lod_count + quadtree_lod, 0);

    let atlas_index      = lookup.x;
    let atlas_lod        = lookup.y;
    let atlas_coordinate = node_coordinate - floor(node_coordinate);

    return NodeLookup(atlas_index, atlas_lod, atlas_coordinate);
}

/*
fn node_size(lod: u32) -> f32 {
    return f32(config.leaf_node_size * (1u << lod));
}

// Looks up the best availale node in the node atlas from the viewers point of view.
// This is done by sampling the viewers quadtree at the caluclated coordinate.
fn lookup_node(lod: u32, local_position: vec3<f32>) -> NodeLookup {
#ifdef SHOW_NODES
    var quadtree_lod = 0u;
    for (; quadtree_lod < config.lod_count; quadtree_lod = quadtree_lod + 1u) {
        let coordinate = local_position.xz / node_size(quadtree_lod);
        let grid_coordinate = floor(view.world_position.xz / node_size(quadtree_lod) + 0.5 - f32(view_config.node_count >> 1u));

        let grid = step(grid_coordinate, coordinate) * (1.0 - step(grid_coordinate + f32(view_config.node_count), coordinate));

        if (grid.x * grid.y == 1.0) {
            break;
        }
    }
#else
    let quadtree_lod = min(lod, config.lod_count - 1u);
#endif

    let quadtree_coords = vec2<i32>((local_position.xz / node_size(quadtree_lod)) % f32(view_config.node_count));
    let lookup = textureLoad(quadtree, quadtree_coords, i32(quadtree_lod), 0);

    let atlas_index = i32(lookup.x);
    let atlas_lod   = lookup.y;
    let atlas_coords = (local_position.xz / node_size(atlas_lod)) % 1.0;

    return NodeLookup(atlas_lod, atlas_index, atlas_coords);
}
*/



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
