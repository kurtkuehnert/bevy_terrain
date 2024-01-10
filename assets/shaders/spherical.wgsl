#import bevy_terrain::types::NodeLookup
#import bevy_terrain::bindings::config
#import bevy_terrain::functions::{vertex_local_position, lookup_node, compute_blend}
#import bevy_terrain::attachments::{sample_attachment0, sample_attachment1, sample_normal, sample_attachment1_gather0}
#import bevy_terrain::vertex::{VertexInput, VertexOutput, vertex_output}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_output}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient2: texture_1d<f32>;
@group(3) @binding(2)
var gradient_sampler: sampler;
@group(3) @binding(3)
var<uniform> material_index: u32;

fn sample_height(lookup: NodeLookup) -> vec2<f32> {
    let gather = sample_attachment1_gather0(lookup);
    let is_valid = all(gather != vec4<f32>(0.0));

    if (is_valid) {
        let height1 = sample_attachment1(lookup).x;
        return vec2<f32>(mix(config.min_height, config.max_height, height1), 1.0);
    }
    else {
        var height0 = sample_attachment0(lookup).x;
        return vec2<f32>(mix(config.min_height, config.max_height, height0), 0.0);
    }
}

fn sample_color(lookup: NodeLookup) -> vec4<f32> {
    let height = sample_height(lookup);

    var color: vec4<f32>;

    if (height.y == 0.0) {
        if (height.x < 0.0) {
            color = textureSampleLevel(gradient, gradient_sampler, mix(0.0, 0.075, pow(height.x / config.min_height, 0.25)), 0.0);
        }
        else {
            color = textureSampleLevel(gradient, gradient_sampler, mix(0.09, 1.0, pow(height.x / config.max_height * 2.0, 1.0)), 0.0);
        }

        if (material_index == 1u) {
            color = vec4<f32>(1.0 - color.x, 1.0 - color.y, 1.0 - color.z, 1.0);
        }
    }
    else {
        let min = -3806.439 / 6371000.0;
        let max = -197.742 / 6371000.0;

        let scale = (height.x - min) / (max - min);

        color = textureSampleLevel(gradient2, gradient_sampler, scale, 0.0);
    }

    return color;
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let local_position = vertex_local_position(input.vertex_index);
    let blend = compute_blend(local_position);

    let lookup = lookup_node(local_position, blend.lod);
    var height = sample_height(lookup).x;

    if (blend.ratio > 0.0) {
        let lookup2 = lookup_node(local_position, blend.lod + 1u);
        height      = mix(height, sample_height(lookup2).x, blend.ratio);
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
