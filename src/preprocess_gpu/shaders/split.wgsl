#import bevy_terrain::preprocessing::{NodeCoordinate, atlas, attachment, inside, pixel_coords, store_entry}

struct AtlasNode {
    coordinate: NodeCoordinate,
    @size(16) atlas_index: u32,
}

struct SplitTileData {
    node: AtlasNode,
    node_index: u32,
}

@group(1) @binding(0)
var<uniform> split_tile_data: SplitTileData;
@group(1) @binding(1)
var tile: texture_2d<f32>;
@group(1) @binding(2)
var tile_sampler: sampler;

fn pixel_value(coords: vec2<u32>) -> f32 {
    if (!inside(coords, vec4<u32>(attachment.border_size, attachment.border_size, attachment.center_size, attachment.center_size))) {
        return 0.0;
    }

    let node_coordinate = split_tile_data.node.coordinate;
    let node_offset =  vec2<f32>(f32(node_coordinate.x), f32(node_coordinate.y));
    let node_coords = vec2<f32>(coords - vec2<u32>(attachment.border_size)) / f32(attachment.center_size);
    let node_scale = f32(1u << (attachment.lod_count - node_coordinate.lod - 1u));

    let tile_coords = (node_offset + node_coords) / node_scale;

    return textureSampleLevel(tile, tile_sampler, tile_coords, 0.0).x;
}

// Todo: respect memory coalescing
@compute @workgroup_size(8, 8, 1)
fn split(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let entry_coords = vec3<u32>(invocation_id.xy, split_tile_data.node_index);

    let entry_value = pack2x16unorm(vec2<f32>(pixel_value(pixel_coords(entry_coords, 0u)),
                                              pixel_value(pixel_coords(entry_coords, 1u))));

    store_entry(entry_coords, entry_value);
}