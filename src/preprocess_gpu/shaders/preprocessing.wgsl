#define_import_path bevy_terrain::preprocessing


struct NodeCoordinate {
    side: u32,
    lod: u32,
    x: u32,
    y: u32,
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

fn inside(coords: vec2<u32>, bounds: vec4<u32>) -> bool {
    return coords.x >= bounds.x &&
           coords.x <  bounds.x + bounds.z &&
           coords.y >= bounds.y &&
           coords.y <  bounds.y + bounds.w;
}

fn pixel_coords(entry_coords: vec3<u32>, pixel_offset: u32) -> vec2<u32> {
    return vec2<u32>(entry_coords.x * attachment.pixels_per_entry + pixel_offset, entry_coords.y);
}

fn store_entry(entry_coords: vec3<u32>, entry_value: u32) {
    let entry_index = entry_coords.z * attachment.entries_per_node +
                      entry_coords.y * attachment.entries_per_side +
                      entry_coords.x;

    atlas_write_section[entry_index] = entry_value;
}
