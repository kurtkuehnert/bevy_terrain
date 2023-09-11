#define_import_path bevy_terrain::minmax

#import bevy_terrain::functions VertexInput,VertexOutput,vertex_output


fn calculate_cube_position(grid_index: u32) -> vec3<f32>{
    var corners = array<vec3<f32>, 14>(
        vec3<f32>( 0.5, -0.5, 1.0),
        vec3<f32>(-0.5, -0.5, 1.0),
        vec3<f32>( 0.5, -0.5, 0.0),
        vec3<f32>(-0.5, -0.5, 0.0),
        vec3<f32>(-0.5,  0.5, 0.0),
        vec3<f32>(-0.5, -0.5, 1.0),
        vec3<f32>(-0.5,  0.5, 1.0),
        vec3<f32>( 0.5, -0.5, 1.0),
        vec3<f32>( 0.5,  0.5, 1.0),
        vec3<f32>( 0.5, -0.5, 0.0),
        vec3<f32>( 0.5,  0.5, 0.0),
        vec3<f32>(-0.5,  0.5, 0.0),
        vec3<f32>( 0.5,  0.5, 1.0),
        vec3<f32>(-0.5,  0.5, 1.0)
    );

    return corners[i32(clamp(grid_index, 1u, 14u)) - 1];
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];

    let size = f32(tile.size) * view_config.tile_scale;
    let center_position = (vec2<f32>(tile.coords) + 0.5) * size;
    let minmax = minmax(center_position, size);


    let cube_position = calculate_cube_position(grid_index);
    let local_position = center_position + cube_position.xy * size;
    let height = mix(minmax.x, minmax.y, cube_position.z);


    var output = vertex_output(local_position, height);

    let color = show_tiles(tile, output.world_position);
    output.debug_color = color;

    return output;

}
