#define_import_path bevy_terrain::vertex

#import bevy_terrain::types NodeLookup, VertexInput, VertexOutput
#import bevy_terrain::bindings view_config, tiles, config, atlas_sampler
#import bevy_terrain::functions calculate_grid_position, calculate_local_position, approximate_world_position, calculate_blend, lookup_node, vertex_output
#import bevy_terrain::debug show_tiles, show_minmax_error
#import bevy_terrain::attachments height_atlas, HEIGHT_SCALE, HEIGHT_OFFSET

// Todo: implement bump mapping, etc.
fn vertex_height(lookup: NodeLookup) -> f32 {
    let height_coords = lookup.atlas_coords * HEIGHT_SCALE + HEIGHT_OFFSET;
    let height = textureSampleLevel(height_atlas, atlas_sampler, height_coords, lookup.atlas_index, 0.0).x;

    return height * config.height;
}

// The default vertex entry point, which blends the height at the fringe between two lods.
fn vertex_fn(in: VertexInput) -> VertexOutput {
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
