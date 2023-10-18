#import bevy_terrain::types TerrainViewConfig, Tile, TileList, Parameters

struct IndirectBuffer {
    workgroup_count: vec3<u32>,
}

@group(1) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(1) @binding(2)
var<storage, read_write> final_tiles: TileList;
@group(1) @binding(3)
var<storage, read_write> temporary_tiles: TileList;
@group(1) @binding(4)
var<storage, read_write> parameters: Parameters;

@group(3) @binding(0)
var<storage, read_write> indirect_buffer: IndirectBuffer;

@compute @workgroup_size(1, 1, 1)
fn prepare_root() {
    parameters.counter = -1;
    parameters.tile_count = 6u;
    atomicStore(&parameters.child_index, i32(view_config.tile_count - 1u));
    atomicStore(&parameters.final_index, 0);

    var top_tile: Tile;
    top_tile.coordinate = vec3<f32>(0.0, 1.0, 0.0);
    top_tile.u          = vec3<f32>(1.0, 0.0, 0.0);
    top_tile.v          = vec3<f32>(0.0, 0.0, 1.0);
    top_tile.side       = 0u;

    var bottom_tile: Tile;
    bottom_tile.coordinate = vec3<f32>(0.0, 0.0, 0.0);
    bottom_tile.u          = vec3<f32>(0.0, 0.0, 1.0);
    bottom_tile.v          = vec3<f32>(1.0, 0.0, 0.0);
    bottom_tile.side       = 1u;

    var front_tile: Tile;
    front_tile.coordinate = vec3<f32>(0.0, 0.0, 0.0);
    front_tile.u          = vec3<f32>(0.0, 1.0, 0.0);
    front_tile.v          = vec3<f32>(0.0, 0.0, 1.0);
    front_tile.side       = 2u;

    var back_tile: Tile;
    back_tile.coordinate = vec3<f32>(1.0, 0.0, 0.0);
    back_tile.u          = vec3<f32>(0.0, 0.0, 1.0);
    back_tile.v          = vec3<f32>(0.0, 1.0, 0.0);
    back_tile.side       = 3u;

    var left_tile: Tile;
    left_tile.coordinate = vec3<f32>(0.0, 0.0, 0.0);
    left_tile.u          = vec3<f32>(1.0, 0.0, 0.0);
    left_tile.v          = vec3<f32>(0.0, 1.0, 0.0);
    left_tile.side       = 4u;

    var right_tile: Tile;
    right_tile.coordinate = vec3<f32>(0.0, 0.0, 1.0);
    right_tile.u          = vec3<f32>(0.0, 1.0, 0.0);
    right_tile.v          = vec3<f32>(1.0, 0.0, 0.0);
    right_tile.side       = 5u;

    temporary_tiles.data[0] = top_tile;
    temporary_tiles.data[1] = bottom_tile;
    temporary_tiles.data[2] = front_tile;
    temporary_tiles.data[3] = back_tile;
    temporary_tiles.data[4] = left_tile;
    temporary_tiles.data[5] = right_tile;

    indirect_buffer.workgroup_count = vec3<u32>(1u, 1u, 1u);
}

@compute @workgroup_size(1, 1, 1)
fn prepare_next() {
    if (parameters.counter == 1) {
        parameters.tile_count = u32(atomicExchange(&parameters.child_index, i32(view_config.tile_count - 1u)));
    }
    else {
        parameters.tile_count =  view_config.tile_count - 1u - u32(atomicExchange(&parameters.child_index, 0));
    }

    parameters.counter = -parameters.counter;
    indirect_buffer.workgroup_count.x = (parameters.tile_count + 63u) / 64u;
}

@compute @workgroup_size(1, 1, 1)
fn prepare_render() {
    let tile_count = u32(atomicLoad(&parameters.final_index));
    let vertex_count = view_config.vertices_per_tile * tile_count;

    indirect_buffer.workgroup_count = vec3<u32>(vertex_count, 1u, 0u);
}