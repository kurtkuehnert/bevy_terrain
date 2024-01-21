#import bevy_terrain::types::NodeLookup
#import bevy_terrain::bindings::config
#import bevy_terrain::functions::{vertex_local_position, lookup_node, compute_blend}
#import bevy_terrain::attachments::{sample_height_grad, sample_normal_grad, sample_attachment1_grad as sample_albedo_grad}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, lookup_info_fragment, fragment_output}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient_sampler: sampler;

fn sample_color_grad(lookup: NodeLookup) -> vec4<f32> {
#ifdef ALBEDO
    return sample_albedo_grad(lookup);
#else
    let height = sample_height_grad(lookup);

    return textureSampleLevel(gradient, gradient_sampler, pow(height, 0.9), 0.0);
#endif
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    let info = lookup_info_fragment(input.local_position);

    let lookup = lookup_node(info, 0u);
    var normal = sample_normal_grad(lookup, input.local_position);
    var color  = sample_color_grad(lookup);

    if (info.blend_ratio > 0.0) {
        let lookup2 = lookup_node(info, 1u);
        normal      = mix(normal, sample_normal_grad(lookup2, input.local_position), info.blend_ratio);
        color       = mix(color,  sample_color_grad(lookup2),                        info.blend_ratio);
    }

#ifdef LIGHTING
    var pbr_input: PbrInput                 = pbr_input_new();
    pbr_input.material.base_color           = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance          = 0.0;
    pbr_input.frag_coord                    = input.fragment_position;
    pbr_input.world_position                = input.world_position;
    pbr_input.world_normal                  = normal;
    pbr_input.N                             = normal;
    pbr_input.V                             = calculate_view(input.world_position, pbr_input.is_orthographic);
    color = apply_pbr_lighting(pbr_input);
#endif

    return fragment_output(input, color, normal, lookup);
}
