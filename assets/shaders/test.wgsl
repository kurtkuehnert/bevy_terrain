#import bevy_terrain::types::{NodeLookup, LookupInfo}
#import bevy_terrain::bindings::{config, atlas_sampler, attachments, attachment0_atlas, attachment1_atlas}
#import bevy_terrain::functions::{vertex_coordinate, lookup_node, lookup_attachment_group, node_count}
#import bevy_terrain::attachments::{sample_attachment0, sample_attachment1, sample_height_grad, sample_normal_grad, sample_attachment1_gather0}
#import bevy_terrain::vertex::{VertexInput, VertexOutput, vertex_lookup_info, vertex_output}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_lookup_info, fragment_output}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

@group(3) @binding(0)
var gradient1: texture_1d<f32>;
@group(3) @binding(1)
var gradient1_sampler: sampler;
@group(3) @binding(2)
var gradient2: texture_1d<f32>;
@group(3) @binding(3)
var gradient2_sampler: sampler;

fn local_available(lookup: NodeLookup) -> bool {
    let attachment = attachments[1];
    let coordinate = lookup.coordinate * attachment.scale + attachment.offset;
    let gather = textureGather(0, attachment1_atlas, atlas_sampler, coordinate, lookup.index);
    return all(gather != vec4<f32>(0.0));
}

fn sample_height(lookup_global: NodeLookup, lookup_local: NodeLookup, local: bool, offset: vec2<f32>) -> f32 {
    var height: f32;

    if (local) {
        let attachment = attachments[1];
        let coordinate = lookup_local.coordinate * attachment.scale + attachment.offset;
        height = textureSampleLevel(attachment1_atlas, atlas_sampler, coordinate + offset, lookup_local.index, 0.0).x;
    }
    else {
        let attachment = attachments[0];
        let coordinate = lookup_global.coordinate * attachment.scale + attachment.offset;
        height = textureSampleLevel(attachment0_atlas, atlas_sampler, coordinate + offset, lookup_global.index, 0.0).x;
    }

    return mix(config.min_height, config.max_height, height);
}

fn sample_color(lookup_global: NodeLookup, lookup_local: NodeLookup, local: bool) -> vec4<f32> {
    let height = sample_height(lookup_global, lookup_local, local, vec2(0.0));

    var color: vec4<f32>;

    if (local) {
        color = textureSampleLevel(gradient2, gradient2_sampler, mix(0.0, 1.0, height / config.min_height), 0.0);
    } else {
        if (height < 0.0) {
            color = textureSampleLevel(gradient1, gradient1_sampler, mix(0.0, 0.075, pow(height / config.min_height, 0.25)), 0.0);
        } else {
            color = textureSampleLevel(gradient1, gradient1_sampler, mix(0.09, 1.0, pow(height / config.max_height * 2.0, 1.0)), 0.0);
        }
    }

    return color;
}

fn sample_normal(lookup_global: NodeLookup, lookup_local: NodeLookup, local: bool, vertex_normal: vec3<f32>, side: u32) -> vec3<f32> {
    let height_attachment = attachments[0];

    var pixels_per_side: f32;

    if (local) { pixels_per_side = height_attachment.size * node_count(lookup_local.lod); }
    else       { pixels_per_side = height_attachment.size * node_count(lookup_global.lod); }


#ifdef SPHERICAL
    var FACE_UP = array(
        vec3( 0.0, 1.0,  0.0),
        vec3( 0.0, 1.0,  0.0),
        vec3( 0.0, 0.0, -1.0),
        vec3( 0.0, 0.0, -1.0),
        vec3(-1.0, 0.0,  0.0),
        vec3(-1.0, 0.0,  0.0),
    );

    let face_up = FACE_UP[side];

    let normal    = normalize(vertex_normal);
    let tangent   = cross(face_up, normal);
    let bitangent = cross(normal, tangent);
    let TBN       = mat3x3(tangent, bitangent, normal);

    let side_length = 3.14159265359 / 4.0;
#else
    let TBN = mat3x3(1.0, 0.0, 0.0,
                     0.0, 0.0, 1.0,
                     0.0, 1.0, 0.0);

    let side_length = 1.0;
#endif

    // Todo: this is only an approximation of the S2 distance (pixels are not spaced evenly and they are not perpendicular)
    let distance_between_samples = side_length / pixels_per_side;
    let offset = 0.5 / height_attachment.size;

    let left  = sample_height(lookup_global, lookup_local, local, vec2<f32>(-offset,     0.0));
    let up    = sample_height(lookup_global, lookup_local, local, vec2<f32>(    0.0, -offset));
    let right = sample_height(lookup_global, lookup_local, local, vec2<f32>( offset,     0.0));
    let down  = sample_height(lookup_global, lookup_local, local, vec2<f32>(    0.0,  offset));

    let surface_normal = normalize(vec3<f32>(left - right, down - up, distance_between_samples));

    return normalize(TBN * surface_normal);
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let info = vertex_lookup_info(input);

    let lookup_global = lookup_attachment_group(info, 0u, 0u);
    let lookup_local  = lookup_attachment_group(info, 0u, 1u);
    let local         = local_available(lookup_local);
    var height        = sample_height(lookup_global, lookup_local, local, vec2(0.0));

    if (info.blend_ratio > 0.0) {
        let lookup_global2 = lookup_attachment_group(info, 1u, 0u);
        let lookup_local2  = lookup_attachment_group(info, 1u, 1u);
        height             = mix(height, sample_height(lookup_global2, lookup_local2, local, vec2(0.0)), info.blend_ratio);
    }

    return vertex_output(input, info, height);
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    let info = fragment_lookup_info(input);

    let lookup_global = lookup_attachment_group(info, 0u, 0u);
    let lookup_local  = lookup_attachment_group(info, 0u, 1u);
    let local = local_available(lookup_local);
    var normal        = sample_normal(lookup_global, lookup_local, local, input.world_normal, input.side);
    var color         = sample_color(lookup_global, lookup_local, local);

    if (info.blend_ratio > 0.0) {
        let lookup_global2 = lookup_attachment_group(info, 1u, 0u);
        let lookup_local2  = lookup_attachment_group(info, 1u, 1u);
        color              = mix(color,  sample_color(lookup_global2, lookup_local2, local),                 info.blend_ratio);
        normal             = mix(normal, sample_normal(lookup_global2, lookup_local2, local, input.world_normal, input.side), info.blend_ratio);
    }

    return fragment_output(input, color, normal, lookup_global);
}
