#import bevy_terrain::types NodeLookup
#import bevy_terrain::bindings config
#import bevy_terrain::functions node_count

@group(2) @binding(2)
var atlas_sampler: sampler;

fn sample_height(lookup: NodeLookup) -> f32 {
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let height = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x - 1.0;

    return config.height * height;
}

// Todo: fix this faulty implementation
fn sample_normal(lookup: NodeLookup, local_position: vec3<f32>) -> vec3<f32> {
#ifdef SPHERICAL
    let normal = normalize(local_position);
    let tangent = cross(vec3(0.0, 1.0, 0.0), normal);
    let bitangent = -cross(tangent, normal);
    let TBN = mat3x3<f32>(tangent, bitangent, normal);

    // Todo: this is only an approximation of the S2 distance
    // local circumference / pixels per circumference
    let distance_between_samples = 3.14159265359 / (4.0 * HEIGHT_SIZE * node_count(lookup.atlas_lod));
#else
    let TBN = mat3x3<f32>(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0);

    let distance_between_samples = 1.0 / (HEIGHT_SIZE * node_count(lookup.atlas_lod));
#endif

    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;

    let left  = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>(-1,  0)).x - 1.0;
    let up    = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>( 0, -1)).x - 1.0;
    let right = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>( 1,  0)).x - 1.0;
    let down  = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>( 0,  1)).x - 1.0;

    let surface_normal = normalize(vec3<f32>(right - left, down - up, 2.0 / config.height * distance_between_samples));

    return normalize(TBN * surface_normal);
}

fn sample_color(lookup: NodeLookup) -> vec4<f32> {
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let height = textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x;

    return vec4<f32>(height * 0.5);
}
