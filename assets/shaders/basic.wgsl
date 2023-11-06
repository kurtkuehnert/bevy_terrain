#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, NodeLookup
#import bevy_terrain::bindings config, atlas_sampler
#import bevy_terrain::functions vertex_local_position, vertex_blend, lookup_node, compute_blend, local_to_world_position
#import bevy_terrain::attachments sample_height, sample_normal, sample_color
#import bevy_terrain::debug show_tiles, debug_color
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::pbr_functions PbrInput, pbr_input_new, calculate_view, pbr

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let local_position = vertex_local_position(input.vertex_index);
    let blend = vertex_blend(local_position);

    let lookup = lookup_node(local_position, blend.lod);
    var height = sample_height(lookup);

    if (blend.ratio < 1.0) {
        let lookup2 = lookup_node(local_position, blend.lod + 1u);
        height      = mix(height, sample_height(lookup2), blend.ratio);
    }

    let world_position = local_to_world_position(local_position, height);

    var output: VertexOutput;
    output.fragment_position = view.view_proj * world_position;
    output.local_position    = local_position;
    output.world_position    = world_position;
    output.debug_color       = show_tiles(input.vertex_index, world_position);

    return output;
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    let blend = compute_blend(input.world_position);

    let lookup = lookup_node(input.local_position, blend.lod);
    var height = sample_height(lookup);
    var normal = sample_normal(lookup, input.local_position);
    var color  = sample_color(lookup);

    if (blend.ratio < 1.0) {
        let lookup2 = lookup_node(input.local_position, blend.lod + 1u);
        height      = mix(height, sample_height(lookup2),                       blend.ratio);
        normal      = mix(normal, sample_normal(lookup2, input.local_position), blend.ratio);
        color       = mix(color,  sample_color(lookup2),                        blend.ratio);
    }

#ifdef LIGHTING
    var pbr_input: PbrInput                 = pbr_input_new();
    pbr_input.material.base_color           = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance          = 0.0;
    pbr_input.frag_coord                    = input.fragment_position;
    pbr_input.world_position                = input.world_position;
    pbr_input.world_normal                  = normal;
    pbr_input.is_orthographic               = view.projection[3].w == 1.0;
    pbr_input.N                             = normal;
    pbr_input.V                             = calculate_view(input.world_position, pbr_input.is_orthographic);
    color = pbr(pbr_input);
#endif

    color = debug_color(color, input, lookup, 0.8);

    return FragmentOutput(color);
}
