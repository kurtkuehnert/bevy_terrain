#define_import_path bevy_terrain::vertex

#import bevy_terrain::types::LookupInfo
#import bevy_terrain::functions::{vertex_local_position, compute_blend, lookup_node, s2_from_local_position, local_to_world_position, local_position_apply_height, world_to_clip_position}
#import bevy_terrain::attachments::sample_height
#import bevy_terrain::debug::show_tiles

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index)   vertex_index: u32,
}

struct VertexOutput {
    @builtin(position)       fragment_position: vec4<f32>,
    @location(0)             local_position: vec3<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}

fn lookup_info_vertex(local_position: vec3<f32>) -> LookupInfo {
    let s2    = s2_from_local_position(local_position);
    let blend = compute_blend(local_position);

    return LookupInfo(s2, blend.lod, blend.ratio, vec2<f32>(0.0), vec2<f32>(0.0));
}

fn vertex_output(input: VertexInput, local_position: vec3<f32>, height: f32) -> VertexOutput {
    var output: VertexOutput;

    output.local_position    = local_position_apply_height(local_position, height);
    output.world_position    = local_to_world_position(output.local_position);
    output.fragment_position = world_to_clip_position(output.world_position);

#ifdef SHOW_TILES
    output.debug_color       = show_tiles(input.vertex_index, output.local_position);
#endif

    return output;
}

fn default_vertex(input: VertexInput) -> VertexOutput {
    let local_position = vertex_local_position(input.vertex_index);
    let info = lookup_info_vertex(local_position);

    let lookup = lookup_node(info, 0u);
    var height = sample_height(lookup);

    if (info.blend_ratio > 0.0) {
        let lookup2 = lookup_node(info, 1u);
        height      = mix(height, sample_height(lookup2), info.blend_ratio);
    }

    return vertex_output(input, local_position, height);
}
