#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile
#import bevy_terrain::bindings config, view_config, tiles, atlas_sampler
#import bevy_terrain::functions vertex_local_position, approximate_world_position
#import bevy_terrain::debug lod_color, show_tiles
#import bevy_pbr::mesh_view_bindings view

@group(3) @binding(0)
var cube_map: texture_cube<f32>;
@group(3) @binding(1)
var gradient: texture_1d<f32>;

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];

    let local_position = vertex_local_position(tile, grid_index);
    var world_position = approximate_world_position(local_position);

    let direction = normalize(local_position);
    let height = 20.0 * pow(textureSampleLevel(cube_map, atlas_sampler, direction, 0.0).x, 0.2);



    world_position = world_position + vec4<f32>(direction * height, 0.0);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = local_position;
    output.world_position = world_position;
    output.debug_color = show_tiles(tile, output.world_position);

    // output.debug_color = lod_color(tile.side);

    return output;
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    let direction = normalize(in.local_position);
    let height = pow(textureSample(cube_map, atlas_sampler, direction).x, 0.62);
    let color = textureSample(gradient, atlas_sampler, height);


    // return FragmentOutput(color);
    return FragmentOutput(in.debug_color);
}
