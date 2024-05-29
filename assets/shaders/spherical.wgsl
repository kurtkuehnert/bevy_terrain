#import bevy_terrain::types::NodeLookup
#import bevy_terrain::bindings::config
#import bevy_terrain::functions::{vertex_coordinate, lookup_node}
#import bevy_terrain::attachments::{sample_height_grad, sample_normal_grad}
#import bevy_terrain::vertex::{VertexInput, VertexOutput, vertex_lookup_info, vertex_output}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_lookup_info, fragment_output}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient_sampler: sampler;
@group(3) @binding(2)
var<uniform> super_elevation: f32;

fn sample_color_grad(lookup: NodeLookup) -> vec4<f32> {
    let height = sample_height_grad(lookup);

    var color: vec4<f32>;

    if (height < 0.0) {
        color = textureSampleLevel(gradient, gradient_sampler, mix(0.0, 0.075, pow(height / config.min_height, 0.25)), 0.0);
    }
    else {
        color = textureSampleLevel(gradient, gradient_sampler, mix(0.09, 1.0, pow(height / config.max_height * 2.0, 1.0)), 0.0);
    }

    return color;
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let info = vertex_lookup_info(input);

     return vertex_output(input, info, 0.0);
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    let info = fragment_lookup_info(input);

    let lookup = lookup_node(info, 0u);
    var normal = sample_normal_grad(lookup, input.world_normal, input.side);
    var color  = sample_color_grad(lookup);

    if (info.blend_ratio > 0.0) {
        let lookup2 = lookup_node(info, 1u);
        normal      = mix(normal, sample_normal_grad(lookup2, input.world_normal, input.side), info.blend_ratio);
        color       = mix(color,  sample_color_grad(lookup2),                                  info.blend_ratio);
    }

    return fragment_output(input, color, normal, lookup);
}
