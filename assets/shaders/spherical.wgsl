#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile
#import bevy_terrain::bindings config, view_config, tiles
#import bevy_terrain::functions vertex_local_position, approximate_world_position, morph
#import bevy_pbr::mesh_view_bindings view

fn lod_color(lod: u32) -> vec4<f32> {
    if (lod % 6u == 0u) {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if (lod % 6u == 1u) {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 2u) {
        return vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 3u) {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 4u) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 5u) {
        return vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    return vec4<f32>(0.0);
}

fn show_tiles(tile: Tile, world_position: vec4<f32>) -> vec4<f32> {
    var color: vec4<f32>;

    let size = length(tile.v);

    let index = ((tile.coordinate.x + tile.coordinate.y + tile.coordinate.z) / size) % 2.0;

    if (index < 0.1) {
        color = vec4<f32>(0.5, 0.5, 0.5, 1.0);
    }
    else {
        color = vec4<f32>(0.1, 0.1, 0.1, 1.0);
    }

    let lod = u32(ceil(log2(1.0 / size)));
    color = mix(color, lod_color(lod), 0.5);
    color = mix(color, lod_color(tile.side), 0.5);

#ifdef MESH_MORPH
    let morph = morph(tile, world_position);
    color = color + vec4<f32>(0.3) * morph;
#endif

    return vec4<f32>(color.xyz, 0.5);
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];

    let local_position = vertex_local_position(tile, grid_index);
    var world_position = approximate_world_position(local_position);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = local_position;
    output.world_position = world_position;
    // output.debug_color = show_tiles(tile, output.world_position);

    output.debug_color = lod_color(tile.side);

    return output;
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    return FragmentOutput(in.debug_color);
}
