#import bevy_terrain::preprocessing::{NodeCoordinate, atlas, attachment, inside, pixel_coords, pixel_value, process_entry}

const INVALID_ATLAS_INDEX: u32 = 4294967295u;

struct AtlasNode {
    coordinate: NodeCoordinate,
    @size(16) atlas_index: u32,
}

struct StitchData {
    node: AtlasNode,
    neighbour_nodes: array<AtlasNode, 8u>,
    node_index: u32,
}

@group(1) @binding(0)
var<uniform> stitch_data: StitchData;

fn project_to_side(coords: vec2<u32>, original_side: u32, projected_side: u32) -> vec2<u32> {
    let PS = 0u;
    let PT = 1u;
    let NS = 2u;
    let NT = 3u;

    var EVEN_LIST = array<vec2<u32>, 6u>(
        vec2<u32>(PS, PT),
        vec2<u32>(PS, PT),
        vec2<u32>(NT, PS),
        vec2<u32>(NT, NS),
        vec2<u32>(PT ,NS),
        vec2<u32>(PS, PT),
    );
    var ODD_LIST = array<vec2<u32>, 6u>(
        vec2<u32>(PS, PT),
        vec2<u32>(PS, PT),
        vec2<u32>(PT, NS),
        vec2<u32>(PT, PS),
        vec2<u32>(NT, PS),
        vec2<u32>(PS, PT),
    );

    let index = (6u + projected_side - original_side) % 6u;
    let info: vec2<u32> = select(ODD_LIST[index], EVEN_LIST[index], original_side % 2u == 0u);

    var neighbour_coords: vec2<u32>;

    if (info.x == PS)      { neighbour_coords.x =                                coords.x; }
    else if (info.x == PT) { neighbour_coords.x =                                coords.y; }
    else if (info.x == NS) { neighbour_coords.x = attachment.texture_size - 1u - coords.x; }
    else if (info.x == NT) { neighbour_coords.x = attachment.texture_size - 1u - coords.y; }

    if (info.y == PS)      { neighbour_coords.y =                                coords.x; }
    else if (info.y == PT) { neighbour_coords.y =                                coords.y; }
    else if (info.y == NS) { neighbour_coords.y = attachment.texture_size - 1u - coords.x; }
    else if (info.y == NT) { neighbour_coords.y = attachment.texture_size - 1u - coords.y; }

    return neighbour_coords;
}

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

fn neighbour_data(coords: vec2<u32>, neighbour_index: u32) -> vec4<f32> {
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

    let neighbour_node = stitch_data.neighbour_nodes[neighbour_index];
    let neighbour_coords = project_to_side(vec2<u32>(vec2<i32>(coords) + offsets[neighbour_index]),
                                           stitch_data.node.coordinate.side,
                                           neighbour_node.coordinate.side);

    return textureLoad(atlas, neighbour_coords, neighbour_node.atlas_index, 0);
}

fn repeat_data(coords: vec2<u32>) -> vec4<f32> {
    let repeat_coords = clamp(coords, vec2<u32>(attachment.border_size),
                                      vec2<u32>(attachment.border_size + attachment.center_size - 1u));

    return textureLoad(atlas, repeat_coords, stitch_data.node.atlas_index, 0);
}

override fn pixel_value(coords: vec2<u32>) -> vec4<f32> {
    if (inside(coords, vec4<u32>(attachment.border_size, attachment.border_size, attachment.center_size, attachment.center_size))) {
        return textureLoad(atlas, coords, stitch_data.node.atlas_index, 0);
    }

    let neighbour_index = neighbour_index(coords);

    if (stitch_data.neighbour_nodes[neighbour_index].atlas_index == INVALID_ATLAS_INDEX) {
        return repeat_data(coords);
    }
    else {
        return neighbour_data(coords, neighbour_index);
    }
}

// Todo: respect memory coalescing
@compute @workgroup_size(8, 8, 1)
fn stitch(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    process_entry(vec3<u32>(invocation_id.xy, stitch_data.node_index));
}