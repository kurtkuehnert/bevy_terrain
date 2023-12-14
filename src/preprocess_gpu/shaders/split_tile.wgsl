struct NodeCoordinate {
    side: u32,
    lod: u32,
    x: u32,
    y: u32,
}

struct NodeMeta {
    node_coordinate: NodeCoordinate,
    @size(16) atlas_index: u32,
}

struct SplitTileData {
    node_meta: NodeMeta,
    node_index: u32,
}

struct AttachmentMeta {
    texture_size: u32,
    border_size: u32,
    node_size: u32,
    pixels_per_entry: u32,
    entries_per_side: u32,
    entries_per_node: u32,
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
var<uniform> split_tile_data: SplitTileData;
@group(1) @binding(1)
var tile: texture_2d<f32>;
@group(1) @binding(2)
var tile_sampler: sampler;

fn inside(coords: vec2<u32>, bounds: vec4<u32>) -> bool {
    return coords.x >= bounds.x &&
           coords.x <  bounds.x + bounds.z &&
           coords.y >= bounds.y &&
           coords.y <  bounds.y + bounds.w;
}

fn tile_value(coords: vec2<u32>) -> f32 {
    if (!inside(coords, vec4<u32>(attachment.border_size, attachment.border_size, attachment.node_size, attachment.node_size))) {
        return 0.0;
    }

    let lod_count = 3u;

    let node_coordinate = split_tile_data.node_meta.node_coordinate;
    let node_offset =  vec2<f32>(f32(node_coordinate.x), f32(node_coordinate.y));
    let node_coords = vec2<f32>(coords - vec2<u32>(attachment.border_size)) / f32(attachment.node_size);
    let node_scale = f32(1u << (lod_count - node_coordinate.lod - 1u));

    let tile_coords = (node_offset + node_coords) / node_scale;

    return textureSampleLevel(tile, tile_sampler, tile_coords, 0.0).x;
}

// Todo: respect memory coalescing
@compute @workgroup_size(8, 8, 1)
fn split_tile(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let entry_coords = vec3<u32>(invocation_id.xy, split_tile_data.node_index);
    let entry_index = entry_coords.z * attachment.entries_per_node +
                      entry_coords.y * attachment.entries_per_side +
                      entry_coords.x;

    let value = pack2x16unorm(vec2<f32>(tile_value(vec2<u32>(entry_coords.x * attachment.pixels_per_entry + 0u, entry_coords.y)),
                                        tile_value(vec2<u32>(entry_coords.x * attachment.pixels_per_entry + 1u, entry_coords.y))));

    atlas_write_section[entry_index] = value;
}