#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile
#import bevy_terrain::bindings config, view_config, tiles, atlas_sampler
#import bevy_terrain::functions vertex_local_position, approximate_world_position, world_position_to_s2_coordinate
#import bevy_terrain::debug lod_color, show_tiles
#import bevy_pbr::mesh_view_bindings view

@group(3) @binding(0)
var cube_map: texture_2d_array<f32>;
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
    // let height = 20.0 * pow(textureSampleLevel(cube_map, atlas_sampler, direction, 0.0).x, 0.2);

    let s2_coordinate = world_position_to_s2_coordinate(world_position);
    let st = s2_coordinate.st;
    let side = s2_coordinate.side;

    let scale = 2.0 * textureSampleLevel(cube_map, atlas_sampler, st, side, 0.0).x - 1.0;
    let height = 40.0 * sign(scale) * pow(abs(scale), 1.5);

    world_position = world_position + vec4<f32>(direction * height, 0.0);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = local_position;
    output.world_position = world_position;
    output.debug_color = show_tiles(tile, output.world_position);

    output.debug_color = lod_color(tile.side);

    return output;
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    let direction = normalize(in.local_position);

    let s2_coordinate = world_position_to_s2_coordinate(in.world_position);
    let st = s2_coordinate.st;
    let side = s2_coordinate.side;

    let scale = 2.0 * textureSampleLevel(cube_map, atlas_sampler, st, side, 0.0).x - 1.0;

    let sample_ocean = textureSample(gradient, atlas_sampler, mix(0.0, 0.075, pow(-scale, 0.25)));
    let sample_land = textureSample(gradient, atlas_sampler, mix(0.09, 1.0, pow(scale * 6.0, 1.75)));

    var color: vec4<f32>;

    if (scale < 0.0) {
        color = sample_ocean;
    }
    else {
        color = sample_land;
    }






    // color = lod_color(side);
    // color = vec4<f32>(st.x, st.y, 0.0, 1.0);

    // color = in.debug_color;

    return FragmentOutput(color);
}
