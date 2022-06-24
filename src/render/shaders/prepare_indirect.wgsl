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
var<storage, read_write> indirect_buffer: IndirectBuffer;
[[group(0), binding(2)]]
var<storage, read_write> parameters: Parameters;

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_tessellation() {
    indirect_buffer.workgroup_count_x = 1u;
    indirect_buffer.workgroup_count_y = 1u;
    indirect_buffer.workgroup_count_z = 1u;

    atomicStore(&parameters.child_index, 0u);
    atomicStore(&parameters.final_index, 0u);
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_refinement() {
    indirect_buffer.workgroup_count_x = atomicExchange(&parameters.child_index, 0u);
    indirect_buffer.workgroup_count_y = 1u;
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_render() {
    indirect_buffer.workgroup_count_x = config.vertices_per_patch * atomicExchange(&parameters.final_index, 0u);
    indirect_buffer.workgroup_count_y = 1u;
    indirect_buffer.workgroup_count_z = 0u;
}