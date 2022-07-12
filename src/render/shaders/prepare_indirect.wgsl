#import bevy_terrain::config
#import bevy_terrain::parameters
#import bevy_terrain::patch

struct IndirectBuffer {
    workgroup_count_x: u32;
    workgroup_count_y: u32;
    workgroup_count_z: u32;
};

[[group(0), binding(0)]]
var<uniform> config: TerrainViewConfig;
[[group(0), binding(2)]]
var<storage, read_write> final_patches: PatchList;
[[group(0), binding(4)]]
var<storage, read_write> parameters: Parameters;

[[group(3), binding(0)]]
var<storage, read_write> indirect_buffer: IndirectBuffer;

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_tessellation() {
    indirect_buffer.workgroup_count_x = 1u;
    indirect_buffer.workgroup_count_y = 1u;
    indirect_buffer.workgroup_count_z = 1u;

    parameters.counter = 1;
    atomicStore(&parameters.child_index, 0);
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_refinement() {
    if (parameters.counter == 1) {
        parameters.counter = -1;
        indirect_buffer.workgroup_count_x = u32(atomicExchange(&parameters.child_index, i32(config.patch_count - 1u)));
    }
    else {
        parameters.counter = 1;
        indirect_buffer.workgroup_count_x = config.patch_count - 1u - u32(atomicExchange(&parameters.child_index, 0));
    }
}

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
    // return atomicExchange(&parameters.final_indices[i32(lod)], 0);
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_render() {
    var vertex_count = 0u;

    for (var i = 0u; i < 4u; i = i + 1u) {
        let patch_size = calc_patch_count(i);
        let vertices_per_row = (patch_size + 2u) << 1u;
        let vertices_per_patch = vertices_per_row * patch_size;

        let first = vertex_count;
        vertex_count = vertex_count + vertices_per_patch * u32(final_index(i));
        let last = vertex_count;

        final_patches.counts[i] = vec2<u32>(first, last);
    }

    indirect_buffer.workgroup_count_x = vertex_count;
    indirect_buffer.workgroup_count_y = 1u;
    indirect_buffer.workgroup_count_z = 0u;
}