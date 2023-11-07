#define_import_path bevy_terrain::fragment

#import bevy_terrain::types NodeLookup
#import bevy_terrain::functions compute_blend, lookup_node
#import bevy_terrain::attachments sample_normal, sample_color
#import bevy_terrain::debug show_lod, show_quadtree
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::pbr_functions PbrInput, pbr_input_new, calculate_view, pbr

struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       fragment_position: vec4<f32>,
    @location(0)             local_position: vec3<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}

struct FragmentOutput {
    @location(0)             color: vec4<f32>
}

fn fragment_output(input: FragmentInput, color: vec4<f32>, lookup: NodeLookup) -> FragmentOutput {
    var output: FragmentOutput;

    output.color = color;

#ifdef SHOW_LOD
    output.color = show_lod(input.local_position, lookup.atlas_lod);
#endif
#ifdef SHOW_UV
    output.color = vec4<f32>(lookup.atlas_coordinate, 0.0, 1.0);
#endif
#ifdef SHOW_TILES
    output.color = input.debug_color;
#endif
#ifdef SHOW_QUADTREE
    output.color = show_quadtree(input.local_position);
#endif

    return output;
}

@fragment
fn default_fragment(input: FragmentInput) -> FragmentOutput {
    let blend = compute_blend(input.local_position);

    let lookup = lookup_node(input.local_position, blend.lod);
    var normal = sample_normal(lookup, input.local_position);
    var color  = sample_color(lookup);

    if (blend.ratio > 0.0) {
        let lookup2 = lookup_node(input.local_position, blend.lod + 1u);
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

    return fragment_output(input, color, lookup);
}
