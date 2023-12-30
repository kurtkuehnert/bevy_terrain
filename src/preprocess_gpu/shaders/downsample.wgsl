#import bevy_terrain::preprocessing::{NodeCoordinate, atlas, attachment, inside, pixel_coords, pixel_value, process_entry}

// Todo: move this once naga oil does not mess up the size attribute anymore
struct AtlasNode {
    coordinate: NodeCoordinate,
    @size(16) atlas_index: u32,
}

struct DownsampleData {
    node: AtlasNode,
    parent_nodes: array<AtlasNode, 4u>,
    node_index: u32,
}

@group(1) @binding(0)
var<uniform> downsample_data: DownsampleData;

override fn pixel_value(coords: vec2<u32>) -> vec4<f32> {
    if (!inside(coords, vec4<u32>(attachment.border_size, attachment.border_size, attachment.center_size, attachment.center_size))) {
        return vec4<f32>(0.0);
    }

    let node_coords = coords - vec2<u32>(attachment.border_size);
    let parent_size = attachment.center_size / 2u;
    let parent_coords = 2u * (node_coords % parent_size) + vec2<u32>(attachment.border_size);
    let parent_index  = node_coords.x / parent_size + 2u * (node_coords.y / parent_size);

    let parent_node = downsample_data.parent_nodes[parent_index];

    return (textureLoad(atlas, parent_coords + vec2<u32>(0u, 0u), parent_node.atlas_index, 0) +
            textureLoad(atlas, parent_coords + vec2<u32>(0u, 1u), parent_node.atlas_index, 0) +
            textureLoad(atlas, parent_coords + vec2<u32>(1u, 0u), parent_node.atlas_index, 0) +
            textureLoad(atlas, parent_coords + vec2<u32>(1u, 1u), parent_node.atlas_index, 0) ) / 4.0;
}

// Todo: respect memory coalescing
@compute @workgroup_size(8, 8, 1)
fn downsample(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    process_entry(vec3<u32>(invocation_id.xy, downsample_data.node_index));
}