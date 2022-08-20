#import bevy_terrain::types
#import bevy_terrain::parameters

struct IndirectBuffer {
    workgroup_count: vec3<u32>,
}

@group(0) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(0) @binding(2)
var<storage, read_write> final_tiles: TileList;
@group(0) @binding(3)
var<storage, read_write> temporary_tiles: TemporaryTileList;
@group(0) @binding(4)
var<storage, read_write> parameters: Parameters;

@group(3) @binding(0)
var<storage, read_write> indirect_buffer: IndirectBuffer;


fn final_index(lod: u32) -> i32 {
    if (lod == 0u) {
        return atomicExchange(&parameters.final_index1, 0);
    }
    if (lod == 1u) {
        return atomicExchange(&parameters.final_index2, 0);
    }
    if (lod == 2u) {
        return atomicExchange(&parameters.final_index3, 0);
    }
    if (lod == 3u) {
        return atomicExchange(&parameters.final_index4, 0);
    }

    return 0;
    // return atomicExchange(&atomics.final_indices[lod], 0);
}

@compute @workgroup_size(1, 1, 1)
fn prepare_tessellation() {
    indirect_buffer.workgroup_count = vec3<u32>(1u, 1u, 1u);

    parameters.counter = 1;
    atomicStore(&parameters.child_index, 1);

    let size = 1u << view_config.refinement_count;

    temporary_tiles.data[0] = TileInfo(vec2<u32>(0u), size);
}

@compute @workgroup_size(1, 1, 1)
fn prepare_refinement() {
    var refinement_count: u32;

    if (parameters.counter == 1) {
        parameters.counter = -1;
        refinement_count = u32(atomicExchange(&parameters.child_index, i32(view_config.tile_count - 1u)));
    }
    else {
        parameters.counter = 1;
        refinement_count =  view_config.tile_count - 1u - u32(atomicExchange(&parameters.child_index, 0));
    }

    parameters.refinement_count = refinement_count;
    indirect_buffer.workgroup_count.x = (refinement_count + 63u) / 64u;
    // indirect_buffer.workgroup_count.x = refinement_count;
}

@compute @workgroup_size(1, 1, 1)
fn prepare_render() {
    var vertex_count = 0u;

    for (var i = 0u; i < 4u; i = i + 1u) {
        let tile_size = calc_tile_count(i);
        let vertices_per_row = (tile_size + 2u) << 1u;
        let vertices_per_tile = vertices_per_row * tile_size;

        let first = vertex_count;
        vertex_count = vertex_count + vertices_per_tile * u32(final_index(i));
        let last = vertex_count;

        final_tiles.counts[i] = vec2<u32>(first, last);
    }

    indirect_buffer.workgroup_count = vec3<u32>(vertex_count, 1u, 0u);
}