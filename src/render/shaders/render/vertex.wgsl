#define_import_path bevy_terrain::vertex
#import bevy_terrain::node NodeLookup,approximate_world_position,lookup_node
#import bevy_terrain::functions VertexInput,VertexOutput,vertex_output,calculate_blend,calculate_grid_position,calculate_local_position
#import bevy_terrain::uniforms atlas_sampler,config,height_atlas,minmax_atlas,tiles,view_config,quadtree
#import bevy_pbr::mesh_view_bindings view

// The default vertex entry point, which blends the height at the fringe between two lods.
@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];
    let grid_position = calculate_grid_position(grid_index);

    let local_position = calculate_local_position(tile, grid_position, config.terrain_size);
    let world_position = approximate_world_position(local_position, view_config.approximate_height);

// view_world_position: vec4<f32>, blend_distance: f32, blend_range: f32
    let blend = calculate_blend(world_position);

    let lookup = lookup_node(blend.lod, local_position, config.lod_count, view_config.node_count, quadtree, config.leaf_node_size);
    var height = vertex_height(lookup);

    if blend.ratio < 1.0 {
        let lookup2 = lookup_node(blend.lod + 1u, local_position, config.lod_count, view_config.node_count, quadtree, config.leaf_node_size);
        let height2 = vertex_height(lookup2);
        height = mix(height2, height, blend.ratio);
    }

    var output = vertex_output(local_position, height);

#ifdef SHOW_TILES
    output.debug_color = show_tiles(tile, output.world_position);
#endif

#ifdef SHOW_MINMAX_ERROR
//  lod_count: u32, minmax_atlas: texture_2d_array<f32>, atlas_sampler: sampler, minmax_scale: f32, minmax_offset: f32, quadtree: texture_2d_array<u32>, leaf_node_size: u32
    output.debug_color = show_minmax_error(tile, height, config.lod_count, minmax_atlas, atlas_sampler, config.minmax_scale, config.minmax_offset, quadtree, config.leaf_node_size);
#endif

#ifdef TEST2
    output.debug_color = mix(output.debug_color, vec4<f32>(f32(tile_index) / 1000.0, 0.0, 0.0, 1.0), 0.4);
#endif

    return output;
}

// The function that evaluates the height of the vertex.
// This will happen once or twice (lod fringe).
fn vertex_height(lookup: NodeLookup) -> f32 {
    let height_coords = lookup.atlas_coords * config.height_scale + config.height_offset;
    let height = textureSampleLevel(height_atlas, atlas_sampler, height_coords, lookup.atlas_index, 0.0).x;

    return height * config.height;
}