#import bevy_terrain::types NodeLookup
#import bevy_terrain::bindings config
#import bevy_terrain::functions vertex_local_position, vertex_blend, lookup_node, compute_blend, local_to_world_position, vertex_output, fragment_output
#import bevy_terrain::attachments atlas_sampler, height_atlas, HEIGHT_SCALE, HEIGHT_OFFSET, sample_height, sample_normal, albedo_atlas, ALBEDO_SCALE, ALBEDO_OFFSET
#import bevy_terrain::fragment FragmentInput, FragmentOutput, fragment_output
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::pbr_functions PbrInput, pbr_input_new, calculate_view, pbr

@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient_sampler: sampler;

fn sample_color(lookup: NodeLookup) -> vec4<f32> {
#ifdef ALBEDO
    let albedo_coords = lookup.atlas_coordinate * ALBEDO_SCALE + ALBEDO_OFFSET;
    return textureSample(albedo_atlas, atlas_sampler, albedo_coords, lookup.atlas_index);
#else
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let height = textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x;

    return textureSampleLevel(gradient, gradient_sampler, pow(height, 0.9), 0.0);
#endif
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
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
