#import bevy_terrain::types::NodeLookup
#import bevy_terrain::attachments::{sample_attachment0_grad as sample_height_grad, sample_normal_grad, sample_attachment1_grad as sample_albedo_grad}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_info, fragment_output, fragment_lookup_node, fragment_debug}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}

@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient_sampler: sampler;

fn sample_color_grad(lookup: NodeLookup) -> vec4<f32> {
#ifdef ALBEDO
    return sample_albedo_grad(lookup);
#else
    let height = sample_height_grad(lookup).x;

    return textureSampleLevel(gradient, gradient_sampler, pow(height, 0.9), 0.0);
#endif
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    var info = fragment_info(input);

    let lookup = fragment_lookup_node(&info, 0u);
    var color  = sample_color_grad(lookup);
    var normal = sample_normal_grad(lookup, info.world_normal, info.tile.side);

    if (info.blend.ratio > 0.0) {
        let lookup2 = fragment_lookup_node(&info, 1u);
        color       = mix(color,  sample_color_grad(lookup2),                                     info.blend.ratio);
        normal      = mix(normal, sample_normal_grad(lookup2, info.world_normal, info.tile.side), info.blend.ratio);
    }

    var output: FragmentOutput;
    fragment_output(&info, &output, color, normal);
    fragment_debug(&info, &output, lookup, normal);

    return output;
}
