#import bevy_terrain::config
#import bevy_terrain::parameters

struct IndirectBuffer {
    workgroup_count_x: u32;
    workgroup_count_y: u32;
    workgroup_count_z: u32;
};

[[group(0), binding(0)]]
var<uniform> config: TerrainConfig;
[[group(0), binding(1)]]
var<storage, read_write> parameters: Parameters;
[[group(2), binding(0)]]
var<storage, read_write> indirect_buffer: IndirectBuffer;

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_tessellation() {
    indirect_buffer.workgroup_count_x = 1u;
    indirect_buffer.workgroup_count_y = 1u;
    indirect_buffer.workgroup_count_z = 1u;

    parameters.counter = 1;
    atomicStore(&parameters.child_index, 0);
    atomicStore(&parameters.final_index, 0);
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

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_render() {
    indirect_buffer.workgroup_count_x = config.vertices_per_patch * u32(atomicExchange(&parameters.final_index, 0));
    indirect_buffer.workgroup_count_y = 1u;
    indirect_buffer.workgroup_count_z = 0u;
}