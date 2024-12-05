struct PickingData {
    cursor_coords: vec2<f32>,
    depth: f32,
}

@group(0) @binding(0)
var<storage, read_write> picking_data: PickingData;
@group(0) @binding(1)
var depth_texture: texture_depth_multisampled_2d;

@compute @workgroup_size(1, 1, 1)
fn pick(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let coords = vec2<f32>(picking_data.cursor_coords.x, 1.0 - picking_data.cursor_coords.y) * vec2<f32>(textureDimensions(depth_texture));
    picking_data.depth = textureLoad(depth_texture, vec2<u32>(coords), 0);
}