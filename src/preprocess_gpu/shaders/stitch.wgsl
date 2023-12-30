#import bevy_terrain::preprocessing::{NodeCoordinate, atlas, attachment, inside, pixel_coords, pixel_value, process_entry}

const INVALID_ATLAS_INDEX: u32 = 4294967295u;

struct AtlasNode {
    coordinate: NodeCoordinate,
    @size(16) atlas_index: u32,
}

struct StitchNodeData {
    node: AtlasNode,
    neighbour_nodes: array<AtlasNode, 8u>,
    node_index: u32,
}

@group(1) @binding(0)
var<uniform> stitch_node_data: StitchNodeData;

fn neighbour_index(coords: vec2<u32>) -> u32 {
    let center_size   = attachment.center_size;
    let border_size = attachment.border_size;
    let offset_size = attachment.border_size + attachment.center_size;

    var bounds = array<vec4<u32>, 8u>(
        vec4<u32>(border_size,          0u, center_size, border_size),
        vec4<u32>(offset_size, border_size, border_size, center_size),
        vec4<u32>(border_size, offset_size, center_size, border_size),
        vec4<u32>(         0u, border_size, border_size, center_size),
        vec4<u32>(         0u,          0u, border_size, border_size),
        vec4<u32>(offset_size,          0u, border_size, border_size),
        vec4<u32>(offset_size, offset_size, border_size, border_size),
        vec4<u32>(         0u, offset_size, border_size, border_size)
    );

    for (var neighbour_index = 0u; neighbour_index < 8u; neighbour_index += 1u) {
        if (inside(coords, bounds[neighbour_index])) { return neighbour_index; }
    }

    return 0u;
}

fn neighbour_coords(coords: vec2<u32>, neighbour_index: u32) -> vec2<u32> {
    let center_size = i32(attachment.center_size);

    var offsets = array<vec2<i32>, 8u>(
        vec2<i32>(           0,  center_size),
        vec2<i32>(-center_size,            0),
        vec2<i32>(           0, -center_size),
        vec2<i32>( center_size,            0),
        vec2<i32>( center_size,  center_size),
        vec2<i32>(-center_size,  center_size),
        vec2<i32>(-center_size, -center_size),
        vec2<i32>( center_size, -center_size)
    );

    return vec2<u32>(vec2<i32>(coords) + offsets[neighbour_index]);
}

fn repeat_coords(coords: vec2<u32>) -> vec2<u32> {
    return clamp(coords,
                 vec2<u32>(attachment.border_size),
                 vec2<u32>(attachment.border_size + attachment.center_size - 1u));
}

override fn pixel_value(coords: vec2<u32>) -> vec4<f32> {
    if (inside(coords, vec4<u32>(attachment.border_size, attachment.border_size, attachment.center_size, attachment.center_size))) {
        return textureLoad(atlas, coords, stitch_node_data.node.atlas_index, 0);
    }

    let neighbour_index = neighbour_index(coords);
    let neighbour_node = stitch_node_data.neighbour_nodes[neighbour_index];

    if (neighbour_node.atlas_index != INVALID_ATLAS_INDEX) {
        return textureLoad(atlas, neighbour_coords(coords, neighbour_index), neighbour_node.atlas_index, 0);
    }
    else {
        return textureLoad(atlas, repeat_coords(coords), stitch_node_data.node.atlas_index, 0);
    }
}

// Todo: respect memory coalescing
@compute @workgroup_size(8, 8, 1)
fn stitch(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    process_entry(vec3<u32>(invocation_id.xy, stitch_node_data.node_index));
}