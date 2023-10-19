#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile
#import bevy_terrain::bindings config, view_config, tiles
#import bevy_terrain::functions vertex_local_position, approximate_world_position
#import bevy_terrain::debug lod_color, show_tiles
#import bevy_pbr::mesh_view_bindings view

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];

    let local_position = vertex_local_position(tile, grid_index);
    var world_position = approximate_world_position(local_position);

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
