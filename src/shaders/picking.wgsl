struct PickingData {
    cursor_coords: vec2<f32>,
    depth: f32,
    world_position: vec3<f32>,
}

@group(0) @binding(0)
var<storage, read_write> picking_data: PickingData;
@group(0) @binding(1)
var depth_sampler: sampler;
@group(0) @binding(2)
var depth_texture: texture_depth_2d;

@compute @workgroup_size(1, 1, 1)
fn pick(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let depth = textureSampleLevel(depth_texture, depth_sampler, picking_data.cursor_coords, 0.0);
    picking_data.depth = depth;
}