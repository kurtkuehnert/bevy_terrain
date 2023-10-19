#define_import_path bevy_terrain::functions

#import bevy_terrain::bindings config, view_config, quadtree, atlas_sampler
#import bevy_terrain::types Tile, NodeLookup, Blend
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

fn tile_coordinate(tile: Tile, uv: vec2<f32>) -> vec3<f32> {
    return tile.coordinate + tile.u * uv.x + tile.v * uv.y;
}

fn tile_local_position(tile: Tile, uv: vec2<f32>) -> vec3<f32> {
    let coordinate = tile_coordinate(tile, uv);

#ifdef SPHERICAL
    let local_position = calculate_sphere_position(coordinate);
#else
    let local_position = calculate_plane_position(coordinate);
#endif

    return local_position;
}

fn morph_threshold_distance(tile: Tile) -> f32 {
    let size = length(tile.u);

    #ifdef SPHERICAL
        let threshold_distance = size * config.radius * view_config.view_distance;
    #else
        let threshold_distance = size * config.terrain_size * view_config.view_distance;
    #endif

    return threshold_distance;
}

fn morph(tile: Tile, world_position: vec4<f32>) -> f32 {
    let viewer_distance = distance(world_position.xyz, view.world_position.xyz);
    let threshold_distance = 2.0 * morph_threshold_distance(tile);

    return clamp(1.0 - (1.0 - viewer_distance / threshold_distance) / view_config.morph_range, 0.0, 1.0);
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

        // let even_grid_offset = grid_offset & vec2<u32>(4294967294u); // set last bit to zero
        let even_grid_offset = grid_offset - (grid_offset & vec2<u32>(1u)); // set last bit to zero
        let even_grid_uv = vec2<f32>(even_grid_offset) / view_config.grid_size;
        let even_local_position = tile_local_position(tile, even_grid_uv);

        local_position = mix(local_position, even_local_position, morph);
    #endif

    return local_position;
}

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

fn calculate_blend(world_position: vec4<f32>) -> Blend {
    let viewer_distance = distance(world_position.xyz, view.world_position.xyz);
    let log_distance = max(log2(2.0 * viewer_distance / view_config.blend_distance), 0.0);
    let ratio = (1.0 - log_distance % 1.0) / view_config.blend_range;

    return Blend(u32(log_distance), ratio);
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
