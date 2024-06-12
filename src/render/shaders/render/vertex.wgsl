#define_import_path bevy_terrain::vertex


struct VertexInput {
    @builtin(vertex_index)   vertex_index: u32,
}

struct VertexOutput {
    @builtin(position)       clip_position: vec4<f32>,
    @location(0)             side: u32,
    @location(1)             uv: vec2<f32>,
    @location(2)             view_distance: f32,
    @location(3)             world_normal: vec3<f32>,
    @location(4)             world_position: vec4<f32>,
    @location(5)             debug_color: vec4<f32>,
}
