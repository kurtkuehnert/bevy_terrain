#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile, S2Coordinate, NodeLookup
#import bevy_terrain::bindings config, view_config, tiles, atlas_sampler, quadtree
#import bevy_terrain::functions grid_offset, vertex_local_position, approximate_world_position, lookup_node, s2_from_world_position, blend, nodes_per_side, s2_project_to_side, node_coordinate, quadtree_lod, inside_rect, s2_to_world_position, calculate_normal
#import bevy_terrain::debug index_color, show_tiles, show_lod, show_quadtree, quadtree_outlines
#import bevy_terrain::attachments height_atlas, HEIGHT_SIZE, HEIGHT_SCALE, HEIGHT_OFFSET
#import bevy_terrain::vertex vertex_fn
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::pbr_functions PbrInput, pbr_input_new, calculate_view, pbr

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
    let normal = calculate_normal(height_coordinate, lookup.atlas_index, lookup.atlas_lod);

    var color: vec4<f32>;
    let opacity = 1.0;

    color = vec4<f32>(0.3);
    // color = vec4<f32>(normal, 1.0);

#ifdef LIGHTING
    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance = 0.0;
    pbr_input.frag_coord = in.frag_coord;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = normal;
    pbr_input.is_orthographic = view.projection[3].w == 1.0;
    pbr_input.N = normal;
    pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);
    color = pbr(pbr_input);
#endif

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
