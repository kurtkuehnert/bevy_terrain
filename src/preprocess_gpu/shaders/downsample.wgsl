#import bevy_terrain::preprocessing::{NodeCoordinate, atlas, attachment, inside, pixel_coords, store_entry}

// Todo: move this once naga oil does not mess up the size attribute anymore
struct NodeMeta {
    node_coordinate: NodeCoordinate,
    @size(16) atlas_index: u32,
}

struct DownsampleData {
    node_meta: NodeMeta,
    parent_nodes: array<NodeMeta, 4u>,
    node_index: u32,
}

@group(1) @binding(0)
var<uniform> downsample_data: DownsampleData;

fn pixel_value(coords: vec2<u32>) -> f32 {
    if (!inside(coords, vec4<u32>(attachment.border_size, attachment.border_size, attachment.center_size, attachment.center_size))) {
        return 0.0;
    }

    let node_coords = coords - vec2<u32>(attachment.border_size);
    let parent_size = attachment.center_size / 2u;
    let parent_coords = 2u * (node_coords % parent_size) + vec2<u32>(attachment.border_size);
    let parent_index  = node_coords.x / parent_size + 2u * (node_coords.y / parent_size);

    let parent_node = downsample_data.parent_nodes[parent_index];

    return (textureLoad(atlas, parent_coords + vec2<u32>(0u, 0u), parent_node.atlas_index, 0).x +
            textureLoad(atlas, parent_coords + vec2<u32>(0u, 1u), parent_node.atlas_index, 0).x +
            textureLoad(atlas, parent_coords + vec2<u32>(1u, 0u), parent_node.atlas_index, 0).x +
            textureLoad(atlas, parent_coords + vec2<u32>(1u, 1u), parent_node.atlas_index, 0).x ) / 4.0;
}

// Todo: respect memory coalescing
@compute @workgroup_size(8, 8, 1)
fn downsample(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let entry_coords = vec3<u32>(invocation_id.xy, downsample_data.node_index);

    let entry_value = pack2x16unorm(vec2<f32>(pixel_value(pixel_coords(entry_coords, 0u)),
                                              pixel_value(pixel_coords(entry_coords, 1u))));

    store_entry(entry_coords, entry_value);
}