struct NodeUpdates {
    data: array<u32>,
}

@group(0) @binding(0)
var quadtree: texture_storage_2d_array<rgba8uint, read_write>; // consider two 16 bit values
@group(0) @binding(1)
var<storage> node_updates: NodeUpdates;

// Todo: consider increasing the workgroup size and pre sorting the updates for better cache coherenze
@compute @workgroup_size(1, 1, 1)
fn update_quadtree(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let index = invocation_id.x;
    let update = node_updates.data[index];

    let atlas_index = update >> 20u;
    let atlas_lod = (update >> 15u) & 0x1Fu;
    let lod = (update >> 10u) & 0x1Fu;
    let x = (update >> 5u) & 0x1Fu;
    let y = update & 0x1Fu;

    let quadtree_entry = vec4<u32>(atlas_index >> 8u, atlas_index & 0xFFu, atlas_lod,  0u);
    textureStore(quadtree, vec2<i32>(i32(x), i32(y)), i32(lod), quadtree_entry);
}
