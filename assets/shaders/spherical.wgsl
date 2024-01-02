#import bevy_terrain::types::NodeLookup
#import bevy_terrain::bindings::config
#import bevy_terrain::functions::{vertex_local_position, lookup_node, compute_blend}
#import bevy_terrain::attachments::{sample_height, sample_normal, sample_attachment0 as sample_height_unscaled}
#import bevy_terrain::vertex::{VertexInput, VertexOutput, vertex_output}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_output}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient_sampler: sampler;

fn sample_color(lookup: NodeLookup) -> vec4<f32> {
    let height = 2.0 * sample_height_unscaled(lookup).x - 1.0;

    if (height < 0.0) {
        return textureSampleLevel(gradient, gradient_sampler, mix(0.0, 0.075, pow(-height, 0.25)), 0.0);
    }
    else {
        return textureSampleLevel(gradient, gradient_sampler, mix(0.09, 1.0, pow(height * 6.0, 1.75)), 0.0);
    }
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let local_position = vertex_local_position(input.vertex_index);
    let blend = compute_blend(local_position);

    let lookup = lookup_node(local_position, blend.lod);
    var height = sample_height(lookup);

    if (blend.ratio > 0.0) {
        let lookup2 = lookup_node(local_position, blend.lod + 1u);
        height      = mix(height, sample_height(lookup2), blend.ratio);
    }

    return vertex_output(input, local_position, height);
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
    pbr_input.N                             = normal;
    pbr_input.V                             = calculate_view(input.world_position, pbr_input.is_orthographic);
    color = apply_pbr_lighting(pbr_input);
#endif

    return fragment_output(input, color, lookup);
}
