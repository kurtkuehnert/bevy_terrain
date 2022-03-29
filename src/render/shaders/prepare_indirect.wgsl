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
fn prepare_area_list() {
    indirect_buffer.workgroup_count_x = config.area_count.x;
    indirect_buffer.workgroup_count_y = config.area_count.y;
    indirect_buffer.workgroup_count_z = 1u;

    atomicStore(&parameters.child_index, 0u);
    atomicStore(&parameters.final_index, 0u);

    parameters.previous_node_count = 0u;
    parameters.lod = config.lod_count;
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_node_list() {
    indirect_buffer.workgroup_count_x = atomicExchange(&parameters.child_index, 0u);
    indirect_buffer.workgroup_count_y = 1u;

    let node_count = atomicLoad(&parameters.final_index);

    parameters.node_counts[parameters.lod] = node_count - parameters.previous_node_count;
    parameters.previous_node_count = node_count;
    parameters.lod = parameters.lod - 1u;
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_patch_list() {
    indirect_buffer.workgroup_count_x = atomicLoad(&parameters.final_index);

    parameters.node_counts[0] = atomicLoad(&parameters.final_index) - parameters.previous_node_count;
}

[[stage(compute), workgroup_size(1, 1, 1)]]
fn prepare_render() {
    indirect_buffer.workgroup_count_x = config.vertices_per_row * config.patch_size;
    indirect_buffer.workgroup_count_y = atomicExchange(&parameters.patch_index, 0u);
    indirect_buffer.workgroup_count_z = 0u;
}