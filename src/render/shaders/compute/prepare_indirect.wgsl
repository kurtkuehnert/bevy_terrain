#import bevy_terrain::types::{TerrainViewConfig, Tile, Parameters}

struct IndirectBuffer {
    workgroup_count: vec3<u32>,
}

@group(1) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(1) @binding(2)
var<storage, read_write> final_tiles: array<Tile>;
@group(1) @binding(3)
var<storage, read_write> temporary_tiles: array<Tile>;
@group(1) @binding(4)
var<storage, read_write> parameters: Parameters;

@group(3) @binding(0)
var<storage, read_write> indirect_buffer: IndirectBuffer;

@compute @workgroup_size(1, 1, 1)
fn prepare_root() {
    parameters.counter = -1;
    atomicStore(&parameters.child_index, i32(view_config.tile_count - 1u));
    atomicStore(&parameters.final_index, 0);

#ifdef SPHERICAL
    parameters.tile_count = 6u;

    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        temporary_tiles[i] = Tile(i, 0u, vec2<u32>(0u));
    }
#else
    parameters.tile_count = 1u;

    temporary_tiles[0] = Tile(0u, 0u, vec2<u32>(0u));
#endif

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