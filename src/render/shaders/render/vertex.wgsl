#define_import_path bevy_terrain::vertex

#import bevy_terrain::bindings::{view_config}
#import bevy_terrain::types::{S2Coordinate, LookupInfo}
#import bevy_terrain::functions::{vertex_coordinate, local_position_from_coordinate, compute_blend, lookup_node, local_to_world_position, world_to_clip_position}
#import bevy_terrain::attachments::sample_height
#import bevy_terrain::debug::show_tiles

struct VertexInput {
    @builtin(vertex_index)   vertex_index: u32,
}

struct VertexOutput {
    @builtin(position)       fragment_position: vec4<f32>,
    @location(0)             side: u32,
    @location(1)             st: vec2<f32>,
    @location(2)             view_distance: f32,
    @location(3)             world_normal: vec3<f32>,
    @location(4)             world_position: vec4<f32>,
    @location(5)             debug_color: vec4<f32>,
}

fn vertex_lookup_info(input: VertexInput) -> LookupInfo {
    let coordinate     = vertex_coordinate(input.vertex_index);
    let local_position = local_position_from_coordinate(coordinate, view_config.approximate_height);
    let view_distance  = distance(local_position, view_config.view_local_position);
    let blend          = compute_blend(view_distance);

    return LookupInfo(coordinate, view_distance, blend.lod, blend.ratio, vec2<f32>(0.0), vec2<f32>(0.0));
}

fn vertex_output(input: VertexInput, info: LookupInfo, height: f32) -> VertexOutput {
    var output: VertexOutput;

    let local_position = local_position_from_coordinate(info.coordinate, height);

    output.side              = info.coordinate.side;
    output.st                = info.coordinate.st;
    output.view_distance     = info.view_distance;
    output.world_normal      = normalize(local_position);
    output.world_position    = local_to_world_position(local_position);
    output.fragment_position = world_to_clip_position(output.world_position);


#ifdef SHOW_TILES
    output.debug_color       = show_tiles(info.coordinate, input.vertex_index);
#endif

    return output;
}

fn default_vertex(input: VertexInput) -> VertexOutput {
    let info = vertex_lookup_info(input);

    let lookup = lookup_node(info, 0u);
    var height = sample_height(lookup);

    if (info.blend_ratio > 0.0) {
        let lookup2 = lookup_node(info, 1u);
        height      = mix(height, sample_height(lookup2), info.blend_ratio);
    }

     return vertex_output(input, info, height);
}
