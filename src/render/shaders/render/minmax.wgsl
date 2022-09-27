#define_import_path bevy_terrain::minmax

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    let tile_lod = 0u;
    let tile_size = 8u;

    let vertices_per_row = (tile_size + 2u) << 1u;
    let vertices_per_tile = vertices_per_row * tile_size;

    let tile_index  = (vertex.index - tiles.counts[tile_lod].x) / vertices_per_tile + tile_lod * 100000u;
    let vertex_index = (vertex.index - tiles.counts[tile_lod].x) % vertices_per_tile;


    let tile = tiles.data[tile_index];

    let size = f32(tile.size) * view_config.tile_scale;
    let local_position = (vec2<f32>(tile.coords) + 0.5) * size;
    let lod = u32(ceil(log2(size))) + 1u;
    let minmax = minmax(local_position, size);

    var corners = array<vec3<f32>, 14>(
        vec3<f32>( 0.5, -0.5, minmax.y),
        vec3<f32>(-0.5, -0.5, minmax.y),
        vec3<f32>( 0.5, -0.5, minmax.x),
        vec3<f32>(-0.5, -0.5, minmax.x),
        vec3<f32>(-0.5,  0.5, minmax.x),
        vec3<f32>(-0.5, -0.5, minmax.y),
        vec3<f32>(-0.5,  0.5, minmax.y),
        vec3<f32>( 0.5, -0.5, minmax.y),
        vec3<f32>( 0.5,  0.5, minmax.y),
        vec3<f32>( 0.5, -0.5, minmax.x),
        vec3<f32>( 0.5,  0.5, minmax.x),
        vec3<f32>(-0.5,  0.5, minmax.x),
        vec3<f32>( 0.5,  0.5, minmax.y),
        vec3<f32>(-0.5,  0.5, minmax.y)
    );

    let corner = corners[i32(clamp(vertex_index, 1u, 14u)) - 1];

    let local_position = local_position + corner.xy * size;
    let world_position = vec4<f32>(local_position.x, corner.z, local_position.y, 1.0);
    let color = show_tiles(tile, local_position, lod);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = local_position;
    output.world_position = world_position;
    output.color = color;

    return output;

}
