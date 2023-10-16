#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput
#import bevy_terrain::bindings view_config, tiles, config
#import bevy_pbr::mesh_view_bindings view
#import bevy_terrain::functions cube_to_sphere

fn color(lod: u32) -> vec4<f32> {
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

fn calculate_grid_position(grid_index: u32) -> vec2<u32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;

    return vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];
    let grid_position = calculate_grid_position(grid_index); // 0..grid_size
    let grid_coords = vec2<f32>(grid_position) / view_config.grid_size; // 0..1

    var position = tile.coord + tile.u * grid_coords.x + tile.v * grid_coords.y;

    position = cube_to_sphere(position) * 50.0;

    var world_position = vec4<f32>(position.x, position.y, position.z, 1.0);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    // output.local_position = vec2<f32>(local_position);
    output.world_position = world_position;
    // output.debug_color = show_tiles(tile, output.world_position);

    output.debug_color = color(tile.side);

    return output;
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    return FragmentOutput(in.debug_color);
}
