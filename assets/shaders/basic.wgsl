#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile, S2Coordinate, NodeLookup
#import bevy_terrain::bindings config, view_config, tiles, atlas_sampler, quadtree
#import bevy_terrain::functions grid_offset, vertex_local_position, approximate_world_position, lookup_node, s2_from_world_position, blend, nodes_per_side, s2_project_to_side, node_coordinate, quadtree_lod, inside_rect, s2_to_world_position
#import bevy_terrain::debug index_color, show_tiles, show_lod, show_quadtree, quadtree_outlines
#import bevy_terrain::attachments height_atlas, HEIGHT_SIZE, HEIGHT_SCALE, HEIGHT_OFFSET
#import bevy_terrain::vertex vertex_fn
#import bevy_pbr::mesh_view_bindings view

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    return vertex_fn(in);
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    // sample chunked clipmap
    let lod = blend(in.world_position).lod;
    let lookup = lookup_node(in.world_position, lod);
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let height = textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x;

    let is_outline = quadtree_outlines(in.world_position, lod);

    var color: vec4<f32>;
    let opacity = 1.0;

    color = vec4<f32>(height);

#ifdef SHOW_LOD
    color = mix(color, show_lod(in.world_position, lookup.atlas_lod), opacity);
#endif
#ifdef SHOW_UV
    color = mix(color, vec4<f32>(lookup.atlas_coordinate, 0.0, 1.0), opacity);
#endif
#ifdef SHOW_TILES
    color = mix(color, in.debug_color, opacity);
#endif
#ifdef SHOW_QUADTREE
    color = mix(color, show_quadtree(in.world_position), opacity);
#endif

    return FragmentOutput(color);
}
