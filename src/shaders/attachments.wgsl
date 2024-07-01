#define_import_path bevy_terrain::attachments

#import bevy_terrain::types::NodeLookup
#import bevy_terrain::bindings::{config, atlas_sampler, attachments, attachment0_atlas, attachment1_atlas, attachment2_atlas}
#import bevy_terrain::functions::node_count

fn attachment_uv(lookup: NodeLookup, attachment_index: u32) -> vec2<f32> {
    let attachment = attachments[attachment_index];
    return lookup.uv * attachment.scale + attachment.offset;
}

fn sample_attachment0(lookup: NodeLookup) -> vec4<f32> {
    let uv = attachment_uv(lookup, 0u);

    return textureSampleLevel(attachment0_atlas, atlas_sampler, uv, lookup.index, 0.0);
}

fn sample_attachment0_grad(lookup: NodeLookup) -> vec4<f32> {
    let uv = attachment_uv(lookup, 0u);

#ifdef SAMPLE_GRAD
    return textureSampleGrad(attachment0_atlas, atlas_sampler, uv, lookup.index, lookup.ddx, lookup.ddy);
#else
    return textureSampleLevel(attachment0_atlas, atlas_sampler, uv, lookup.index, 0.0);
#endif
}

fn sample_attachment1(lookup: NodeLookup) -> vec4<f32> {
    let uv = attachment_uv(lookup, 1u);

    return textureSampleLevel(attachment1_atlas, atlas_sampler, uv, lookup.index, 0.0);
}

fn sample_attachment1_grad(lookup: NodeLookup) -> vec4<f32> {
    let uv = attachment_uv(lookup, 1u);

#ifdef SAMPLE_GRAD
    return textureSampleGrad(attachment1_atlas, atlas_sampler, uv, lookup.index, lookup.ddx, lookup.ddy);
#else
    return textureSampleLevel(attachment1_atlas, atlas_sampler, uv, lookup.index, 0.0);
#endif
}

fn sample_attachment1_gather0(lookup: NodeLookup) -> vec4<f32> {
    let uv = attachment_uv(lookup, 1u);
    return textureGather(0, attachment1_atlas, atlas_sampler, uv, lookup.index);
}

fn sample_height(lookup: NodeLookup) -> f32 {
    let height = sample_attachment0(lookup).x;

    return mix(config.min_height, config.max_height, height);
}

fn sample_height_grad(lookup: NodeLookup) -> f32 {
    let height = sample_attachment0_grad(lookup).x;

    return mix(config.min_height, config.max_height, height);
}

fn sample_normal_grad(lookup: NodeLookup, vertex_normal: vec3<f32>, side: u32) -> vec3<f32> {
    let uv = attachment_uv(lookup, 0u);

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

    let side_length = 3.14159265359 / 4.0 * config.scale;
#else
    let TBN = mat3x3(1.0, 0.0, 0.0,
                     0.0, 0.0, 1.0,
                     0.0, 1.0, 0.0);

    let side_length = config.scale;
#endif

    // Todo: this is only an approximation of the S2 distance (pixels are not spaced evenly and they are not perpendicular)
    let pixels_per_side = attachments[0u].size * node_count(lookup.lod);
    let distance_between_samples = side_length / pixels_per_side;
    let offset = 0.5 / attachments[0u].size;

#ifdef SAMPLE_GRAD
    let left  = mix(config.min_height, config.max_height, textureSampleGrad(attachment0_atlas, atlas_sampler, uv + vec2<f32>(-offset,     0.0), lookup.index, lookup.ddx, lookup.ddy).x);
    let up    = mix(config.min_height, config.max_height, textureSampleGrad(attachment0_atlas, atlas_sampler, uv + vec2<f32>(    0.0, -offset), lookup.index, lookup.ddx, lookup.ddy).x);
    let right = mix(config.min_height, config.max_height, textureSampleGrad(attachment0_atlas, atlas_sampler, uv + vec2<f32>( offset,     0.0), lookup.index, lookup.ddx, lookup.ddy).x);
    let down  = mix(config.min_height, config.max_height, textureSampleGrad(attachment0_atlas, atlas_sampler, uv + vec2<f32>(    0.0,  offset), lookup.index, lookup.ddx, lookup.ddy).x);
#else
    let left  = mix(config.min_height, config.max_height, textureSampleLevel(attachment0_atlas, atlas_sampler, uv + vec2<f32>(-offset,     0.0), lookup.index, 0.0).x);
    let up    = mix(config.min_height, config.max_height, textureSampleLevel(attachment0_atlas, atlas_sampler, uv + vec2<f32>(    0.0, -offset), lookup.index, 0.0).x);
    let right = mix(config.min_height, config.max_height, textureSampleLevel(attachment0_atlas, atlas_sampler, uv + vec2<f32>( offset,     0.0), lookup.index, 0.0).x);
    let down  = mix(config.min_height, config.max_height, textureSampleLevel(attachment0_atlas, atlas_sampler, uv + vec2<f32>(    0.0,  offset), lookup.index, 0.0).x);
#endif

    let surface_normal = normalize(vec3<f32>(left - right, down - up, distance_between_samples));

    return normalize(TBN * surface_normal);
}

fn sample_color_grad(lookup: NodeLookup) -> vec4<f32> {
    let height = sample_attachment0_grad(lookup).x;

    return vec4<f32>(height * 0.5);
}
