#import bevy_terrain::preprocessing::{NodeCoordinate, atlas, attachment, inside, pixel_coords, pixel_value, process_entry}
#import bevy_terrain::functions::inside_square;

struct AtlasNode {
    coordinate: NodeCoordinate,
    @size(16) atlas_index: u32,
}

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

fn inverse_mix(lower: vec2<f32>, upper: vec2<f32>, value: vec2<f32>) -> vec2<f32> {
    return (value - lower) / (upper - lower);
}

override fn pixel_value(coords: vec2<u32>) -> vec4<f32> {
    if (!inside(coords, vec4<u32>(attachment.border_size, attachment.border_size, attachment.center_size, attachment.center_size))) {
        return vec4<f32>(0.0);
    }

    let node_coordinate = split_data.node.coordinate;
    let node_offset =  vec2<f32>(f32(node_coordinate.x), f32(node_coordinate.y));
    let node_coords = vec2<f32>(coords - vec2<u32>(attachment.border_size)) / f32(attachment.center_size);
    let node_scale = f32(1u << (attachment.lod_count - node_coordinate.lod - 1u));

    var tile_coords = (node_offset + node_coords) / node_scale;

    tile_coords = inverse_mix(split_data.top_left, split_data.bottom_right, tile_coords);

    let value = textureSampleLevel(tile, tile_sampler, tile_coords, 0.0);

    let gather = textureGather(0u, tile, tile_sampler, tile_coords);
    let is_valid = all(gather != vec4<f32>(0.0));

    if ((inside_square(tile_coords, vec2<f32>(0.0), 1.0) == 1.0) && is_valid) {
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