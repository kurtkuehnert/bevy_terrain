#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile
#import bevy_terrain::bindings view_config, tiles, config
#import bevy_pbr::mesh_view_bindings view
#import bevy_terrain::functions cube_to_sphere

fn lod_color(lod: u32) -> vec4<f32> {
    if (lod % 6u == 0u) {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if (lod % 6u == 1u) {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 2u) {
        return vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 3u) {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 4u) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 5u) {
        return vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    return vec4<f32>(0.0);
}

fn show_tiles(tile: Tile, world_position: vec4<f32>) -> vec4<f32> {
    var color: vec4<f32>;

    let size = length(tile.v);

    let index = ((tile.coord.x + tile.coord.y + tile.coord.z) / size) % 2.0;

    if (index < 0.1) {
        color = vec4<f32>(0.5, 0.5, 0.5, 1.0);
    }
    else {
        color = vec4<f32>(0.1, 0.1, 0.1, 1.0);
    }

    let lod = u32(ceil(log2(1.0 / size)));
    color = mix(color, lod_color(lod), 0.5);
    color = mix(color, lod_color(tile.side), 0.5);

#ifdef MESH_MORPH
    let morph = calculate_morph(tile, world_position);
    color = color + vec4<f32>(0.3) * morph;
#endif

    return vec4<f32>(color.xyz, 0.5);
}

fn calculate_morph(tile: Tile, world_position: vec4<f32>) -> f32 {
    let viewer_distance = distance(world_position.xyz, view.world_position.xyz);
    let size = length(tile.u);

    let morph_distance = 2.0 * size * 300.0;

    return clamp(1.0 - (1.0 - viewer_distance / morph_distance) / view_config.morph_range, 0.0, 1.0);
}

fn calculate_grid_position(grid_index: u32) -> vec2<u32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;

    return vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);
}

fn calculate_local_position(tile: Tile, grid_position: vec2<u32>) -> vec3<f32> {
    let grid_coords = vec2<f32>(grid_position) / view_config.grid_size;
    var local_position = tile.coord + tile.u * grid_coords.x + tile.v * grid_coords.y;
        local_position = cube_to_sphere(local_position) * 50.0;

#ifdef MESH_MORPH
    let world_position = vec4<f32>(local_position, 1.0);
    let morph = calculate_morph(tile, world_position);

    let even_grid_position = grid_position - (grid_position & vec2<u32>(1u));
    let even_grid_coords = vec2<f32>(even_grid_position) / view_config.grid_size;
    var even_local_position = tile.coord + tile.u * even_grid_coords.x + tile.v * even_grid_coords.y;
        even_local_position = cube_to_sphere(even_local_position) * 50.0;

    local_position = mix(local_position, even_local_position, morph);
#endif

    return local_position;
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];
    let grid_position = calculate_grid_position(grid_index); // 0..grid_size
    let local_position = calculate_local_position(tile, grid_position);

    var world_position = vec4<f32>(local_position, 1.0);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = local_position;
    output.world_position = world_position;
    output.debug_color = show_tiles(tile, output.world_position);

    // output.debug_color = lod_color(tile.side);

    return output;
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    return FragmentOutput(in.debug_color);
}
