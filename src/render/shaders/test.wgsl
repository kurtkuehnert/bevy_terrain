struct TerrainConfig {
    lod_count: u32,
    height: f32,
    leaf_node_size: u32,
    terrain_size: u32,
}

@group(2) @binding(1)
var<uniform> config: TerrainConfig;
@group(2) @binding(1)
var atlas_sampler: sampler;


@group(2) @binding(2)
var {attachment}_atlas: texture_2d_array<f32>;

const {attachment}_SCALE: f32 = ;


// attachment params
// attachment textures
// Fragment Data struct has to be embedded in the code
// attachment sample function (optional)

struct AttachmentParams {
    size: f32,
    scale: f32,
    offset: f32,
}

var attachment_params: AttachmentParams;

@group(2) @binding(X)
var attachment_atlas: texture_2d_array<f32>;




fn lookup_attachment(lookup: NodeLookup) -> f32 {
    let attachment_coords = lookup.atlas_coords * attachment_params.scale + attachment_params.offset;

    return textureSampleLevel(attachment_atlas, atlas_sampler, attachment_coords, lookup.atlas_index, 0.0).x;
}

fn sample_attachment(local_position) -> AttachmentData {
    let world_position = approximate_world_position(local_position);

    let blend = calculate_blend(world_position);

    let lookup = lookup_node(blend.lod, local_position);
    var attachment_data = lookup_attachment(lookup);

    if (blend.ratio < 1.0) {
        let lookup2 = lookup_node(blend.lod + 1u, local_position);
        attachment_data = mix(lookup_attachment(lookup2), attachment_data, blend.ratio);
    }

    return attachment_data;
}

fn sample_all_attachment(local_position) -> AttachmentData {
    let world_position = approximate_world_position(local_position);

    let blend = calculate_blend(world_position);

    let lookup = lookup_node(blend.lod, local_position);

    var attachment1_data = lookup_attachment1(lookup);
    var attachment2_data = lookup_attachment2(lookup);
    var attachment3_data = lookup_attachment3(lookup);

    if (blend.ratio < 1.0) {
        let lookup2 = lookup_node(blend.lod + 1u, local_position);

        attachment1_data = mix(lookup_attachment1(lookup2), attachment_data1, blend.ratio);
        attachment1_data = mix(lookup_attachment2(lookup2), attachment_data2, blend.ratio);
        attachment1_data = mix(lookup_attachment3(lookup2), attachment_data3, blend.ratio);
    }

    return attachment_data;
}


