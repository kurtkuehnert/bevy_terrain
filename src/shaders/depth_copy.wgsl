#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var depth_texture: texture_depth_multisampled_2d;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @builtin(frag_depth) f32 {
    return textureLoad(depth_texture, vec2<u32>(in.uv * vec2<f32>(textureDimensions(depth_texture))), 0);
}