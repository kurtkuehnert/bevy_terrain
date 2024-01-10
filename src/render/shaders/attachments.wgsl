#define_import_path bevy_terrain::attachments

#import bevy_terrain::types::NodeLookup
#import bevy_terrain::bindings::{config, atlas_sampler, attachments, attachment0_atlas, attachment1_atlas, attachment2_atlas}
#import bevy_terrain::functions::node_count

fn sample_attachment0(lookup: NodeLookup) -> vec4<f32> {
    let attachment = attachments.data[0];
    let coordinate = lookup.atlas_coordinate * attachment.scale + attachment.offset;
    return textureSampleLevel(attachment0_atlas, atlas_sampler, coordinate, lookup.atlas_index, 0.0);
}

fn sample_attachment1(lookup: NodeLookup) -> vec4<f32> {
    let attachment = attachments.data[1];
    let coordinate = lookup.atlas_coordinate * attachment.scale + attachment.offset;
    return textureSampleLevel(attachment1_atlas, atlas_sampler, coordinate, lookup.atlas_index, 0.0);
}

fn sample_attachment2(lookup: NodeLookup) -> vec4<f32> {
    let attachment = attachments.data[2];
    let coordinate = lookup.atlas_coordinate * attachment.scale + attachment.offset;
    return textureSampleLevel(attachment2_atlas, atlas_sampler, coordinate, lookup.atlas_index, 0.0);
}

fn sample_attachment1_gather0(lookup: NodeLookup) -> vec4<f32> {
    let attachment = attachments.data[1];
    let coordinate = lookup.atlas_coordinate * attachment.scale + attachment.offset;
    return textureGather(0, attachment1_atlas, atlas_sampler, coordinate, lookup.atlas_index);
}

fn sample_height(lookup: NodeLookup) -> f32 {
    let height = sample_attachment0(lookup).x;

    return mix(config.min_height, config.max_height, height);
}

// Todo: fix this faulty implementation
fn sample_normal(lookup: NodeLookup, local_position: vec3<f32>) -> vec3<f32> {
    let height_attachment = attachments.data[0];
    let height_coordinate = lookup.atlas_coordinate * height_attachment.scale + height_attachment.offset;

#ifdef SPHERICAL
    let normal = normalize(local_position);
    let tangent = cross(vec3(0.0, 1.0, 0.0), normal);
    let bitangent = -cross(tangent, normal);
    let TBN = mat3x3<f32>(tangent, bitangent, normal);

    // Todo: this is only an approximation of the S2 distance
    // local circumference / pixels per circumference
    let distance_between_samples = 3.14159265359 / (4.0 * height_attachment.size * f32(node_count(lookup.atlas_lod)));
#else
    let TBN = mat3x3<f32>(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0);

    let distance_between_samples = 4.0 * 1.0 / (height_attachment.size * f32(node_count(lookup.atlas_lod)));
#endif

    let left  = mix(config.min_height, config.max_height, textureSampleLevel(attachment0_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>(-1,  0)).x);
    let up    = mix(config.min_height, config.max_height, textureSampleLevel(attachment0_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>( 0, -1)).x);
    let right = mix(config.min_height, config.max_height, textureSampleLevel(attachment0_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>( 1,  0)).x);
    let down  = mix(config.min_height, config.max_height, textureSampleLevel(attachment0_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>( 0,  1)).x);

    let surface_normal = normalize(vec3<f32>(right - left, down - up, distance_between_samples));

    return normalize(TBN * surface_normal);
}

fn sample_color(lookup: NodeLookup) -> vec4<f32> {
    let height = sample_attachment0(lookup).x;

    return vec4<f32>(height * 0.5);
}
