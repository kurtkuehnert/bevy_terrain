#import bevy_terrain::preprocessing::{AtlasNode, atlas, attachment, pixel_coords, pixel_value, process_entry, is_border, inverse_mix}
#import bevy_terrain::functions::{inside_square, node_count};

struct SplitData {
    node: AtlasNode,
    top_left: vec2<f32>,
    bottom_right: vec2<f32>,
    node_index: u32,
}

@group(1) @binding(0)
var<uniform> split_data: SplitData;
@group(1) @binding(1)
var tile: texture_2d<f32>;
@group(1) @binding(2)
var tile_sampler: sampler;

override fn pixel_value(coords: vec2<u32>) -> vec4<f32> {
    if (is_border(coords)) {
        return vec4<f32>(0.0);
    }

    let node_coordinate = split_data.node.coordinate;
    let node_offset =  vec2<f32>(f32(node_coordinate.x), f32(node_coordinate.y));
    let node_coords = vec2<f32>(coords - vec2<u32>(attachment.border_size)) / f32(attachment.center_size);
    let node_scale = node_count(node_coordinate.lod);

    var tile_coords = (node_offset + node_coords) / node_scale;

    tile_coords = inverse_mix(split_data.top_left, split_data.bottom_right, tile_coords);

    let value = textureSampleLevel(tile, tile_sampler, tile_coords, 0.0);

    let is_valid  = all(textureGather(0u, tile, tile_sampler, tile_coords) != vec4<f32>(0.0));
    let is_inside = inside_square(tile_coords, vec2<f32>(0.0), 1.0) == 1.0;

    if (is_valid && is_inside) {
        return value;
    }
    else {
        return textureLoad(atlas, coords, split_data.node.atlas_index, 0);
    }
}

// Todo: respect memory coalescing
@compute @workgroup_size(8, 8, 1)
fn split(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    process_entry(vec3<u32>(invocation_id.xy, split_data.node_index));
}