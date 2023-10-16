#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, NodeLookup
#import bevy_terrain::bindings view_config, tiles, config, atlas_sampler
#import bevy_terrain::functions calculate_grid_position, calculate_local_position, approximate_world_position, calculate_blend, lookup_node, vertex_output
#import bevy_terrain::debug show_tiles, show_minmax_error
#import bevy_terrain::attachments height_atlas, HEIGHT_SCALE, HEIGHT_OFFSET
#import bevy_terrain::fragment fragment_fn

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];
    let grid_position = calculate_grid_position(grid_index);

    let local_position = calculate_local_position(tile, grid_position);
    let world_position = approximate_world_position(local_position);

    let height = 0.0;

    var output = vertex_output(local_position, height);

    output.debug_color = show_tiles(tile, output.world_position);

    return output;
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    return FragmentOutput(in.debug_color);
}
