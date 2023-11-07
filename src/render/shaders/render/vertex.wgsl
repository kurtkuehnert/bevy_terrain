#define_import_path bevy_terrain::vertex

#import bevy_terrain::functions vertex_local_position, compute_blend, lookup_node, local_to_world_position
#import bevy_terrain::attachments sample_height
#import bevy_terrain::debug show_tiles
#import bevy_pbr::mesh_view_bindings view

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

fn vertex_output(input: VertexInput, local_position: vec3<f32>, height: f32) -> VertexOutput {
    let world_position = local_to_world_position(local_position, height);

    var output: VertexOutput;
    output.fragment_position = view.view_proj * world_position;
    output.local_position    = local_position;
    output.world_position    = world_position;

#ifdef SHOW_TILES
    output.debug_color       = show_tiles(input.vertex_index, local_position);
#endif

    return output;
}

fn default_vertex(input: VertexInput) -> VertexOutput {
    let local_position = vertex_local_position(input.vertex_index);
    let blend = compute_blend(local_position);

    let lookup = lookup_node(local_position, blend.lod);
    var height = sample_height(lookup);

    if (blend.ratio > 0.0) {
        let lookup2 = lookup_node(local_position, blend.lod + 1u);
        height      = mix(height, sample_height(lookup2), blend.ratio);
    }

    return vertex_output(input, local_position, height);
}
