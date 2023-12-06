#import bevy_terrain::types::S2Coordinate
#import bevy_terrain::fuctions

struct NodeMeta {
    atlas_index: u32,
   _padding: u32,
    node_coordinate: NodeCoordinate,
}

struct AttachmentMeta {
    texture_size: u32,
    border_size: u32,
    node_size: u32,
    pixels_per_section_entry: u32,
}

struct NodeCoordinate {
    side: u32,
    lod: u32,
    xy: vec2<u32>,
}

struct NodeMetaList {
    data: array<NodeMeta>,
}

@group(0) @binding(0)
var<storage, read_write> atlas_write_section: array<u32>;
@group(0) @binding(1)
var atlas: texture_2d_array<f32>;
@group(0) @binding(2)
var atlas_sampler: sampler;
@group(0) @binding(3)
var<uniform> attachment: AttachmentMeta;

@group(1) @binding(0)
var tile: texture_2d<f32>;
@group(1) @binding(1)
var tile_sampler: sampler;
@group(1) @binding(2)
var<storage> node_meta_list: NodeMetaList;

fn s2_from_node_coordinate(node_coordinate: NodeCoordinate) -> S2Coordinate {
    let st = vec2<f32>(node_coordinate.xy) / f32(1 << node_coordinate.lod); // Todo: this is probably incorrect

    return S2Coordinate(node_coordinate.side, st);
}

fn s2_pixel(node_s2: S2Coordinate, node_uv: vec2<u32>) -> S2Coordinate {
    return node_s2;
}

fn sample_tile(s2: S2Coordinate) -> vec4<u32> {
    return vec4<u32>(60000u);
}

fn tile_value(pixel_coords: vec2<u32>, pixel_offset: u32, node_meta: NodeMeta) -> f32 {
    let node_coordinate = node_meta.node_coordinate;

    let lod_count = 2u;

    let node_scale = f32(1u << (lod_count - node_coordinate.lod - 1u));
    let node_coords = vec2<f32>(pixel_coords + vec2<u32>(pixel_offset, 0u)) / f32(attachment.node_size);
    let node_offset =  vec2<f32>(node_coordinate.xy);

    let tile_coords = (node_offset + node_coords) / node_scale;

    return textureSampleLevel(tile, tile_sampler, tile_coords, 0.0).x;
}

// Todo: respect memory coalescing
@compute @workgroup_size(8, 8, 1)
fn split_tile(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let pixel_coords = vec2<u32>(invocation_id.x * attachment.pixels_per_section_entry, invocation_id.y);
    let node_index = invocation_id.z;

    if (pixel_coords.x >= attachment.node_size || pixel_coords.y >= attachment.node_size) {
        return;
    }

    let node_meta = node_meta_list.data[node_index];

    let value = pack2x16unorm(vec2<f32>(tile_value(pixel_coords, 0u, node_meta),
                                        tile_value(pixel_coords, 1u, node_meta)));

    let section_coords = pixel_coords + vec2<u32>(attachment.border_size);
    let section_index = (node_index * attachment.texture_size * attachment.texture_size +
                         section_coords.y * attachment.texture_size +
                         section_coords.x) / attachment.pixels_per_section_entry;

    atlas_write_section[section_index] = value;

//
//     let node_meta = node_meta_list.data[node_index];
//     let node_coordinate = node_meta.node_coordinate;
//     let atlas_index = node_meta.atlas_index;
//     let node_s2 = s2_from_node_coordinate(node_coordinate);
//     let node_uv = vec2<u32>(pixel_index / node_size + border_size, pixel_index % node_size + border_size);
//
//     let pixel_s2 = s2_pixel(node_s2, node_uv);
//
//     let pixel_value = sample_tile(pixel_s2);
//
//
//     let pixel_value = vec4<u32>(60000u);
}