#define_import_path bevy_terrain::vertex

// The function that evaluates the height of the vertex.
// This will happen once or twice (lod fringe).
// fn vertex_height(lookup: AtlasLookup) -> f32;

// The default vertex entry point, which blends the height at the fringe between two lods.
@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];
    let grid_position = calculate_grid_position(grid_index);

    let local_position = calculate_local_position(tile, grid_position);
    let world_position = approximate_world_position(local_position);

    let blend = calculate_blend(world_position);

    let lookup = lookup_node(blend.lod, local_position);
    var height = vertex_height(lookup);

    if (blend.ratio < 1.0) {
        let lookup2 = lookup_node(blend.lod + 1u, local_position);
        let height2 = vertex_height(lookup2);
        height      = mix(height2, height, blend.ratio);
    }

    var output = vertex_output(local_position, height);

#ifdef SHOW_TILES
    output.debug_color = show_tiles(tile, output.world_position);
#endif

#ifdef SHOW_MINMAX_ERROR
    output.debug_color = show_minmax_error(tile, height);
#endif

#ifdef TEST2
    output.debug_color = mix(output.debug_color, vec4<f32>(f32(tile_index) / 1000.0, 0.0, 0.0, 1.0), 0.4);
#endif

    return output;
}
