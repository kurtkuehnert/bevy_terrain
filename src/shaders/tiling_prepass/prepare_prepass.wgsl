#import bevy_terrain::types::{TileCoordinate, Blend}
#import bevy_terrain::bindings::{terrain_view, approximate_height, temporary_tiles, parameters, indirect_buffer}
#import bevy_terrain::functions::{compute_view_coordinate, lookup_tile, compute_local_position, position_local_to_world, normal_local_to_world}
#import bevy_terrain::attachments::{sample_height, sample_attachment0_gather0}

@compute @workgroup_size(1, 1, 1)
fn prepare_root() {
    parameters.counter = -1;
    atomicStore(&parameters.child_index, i32(terrain_view.tile_count - 1u));
    atomicStore(&parameters.final_index, 0);

#ifdef SPHERICAL
    parameters.tile_count = 6u;

    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        temporary_tiles[i] = TileCoordinate(i, 0u, vec2<u32>(0u));
    }
#else
    parameters.tile_count = 1u;

    temporary_tiles[0] = TileCoordinate(0u, 0u, vec2<u32>(0u));
#endif

    indirect_buffer.workgroup_count = vec3<u32>(1u, 1u, 1u);

    // compute approximate height
    let coordinate = compute_view_coordinate(terrain_view.view_face, terrain_view.view_lod);
    let blend      = Blend(coordinate.lod, 0.0);
    let tile       = lookup_tile(coordinate, blend, 0u);
    let raw_height = sample_attachment0_gather0(tile);
    let mask       = bitcast<vec4<u32>>(raw_height) & vec4<u32>(1);

    if (all(mask != vec4<u32>(0))) {
        approximate_height = sample_height(tile);
    }

    let local_position = compute_local_position(coordinate);
    let world_position = position_local_to_world(local_position);
    let world_normal   = normal_local_to_world(local_position);
    let approximate_position = world_position + world_normal * approximate_height;
    let view_world_position = terrain_view.view_world_position;

    let distance = dot(world_normal, view_world_position) - dot(world_normal, approximate_position);

    if (distance < 1.0) {
       //  parameters.tile_count = 0u;
    }
}

@compute @workgroup_size(1, 1, 1)
fn prepare_next() {
    if (parameters.counter == 1) {
        parameters.tile_count = u32(atomicExchange(&parameters.child_index, i32(terrain_view.tile_count - 1u)));
    }
    else {
        parameters.tile_count = terrain_view.tile_count - 1u - u32(atomicExchange(&parameters.child_index, 0));
    }

    parameters.counter = -parameters.counter;
    indirect_buffer.workgroup_count.x = (parameters.tile_count + 63u) / 64u;
}

@compute @workgroup_size(1, 1, 1)
fn prepare_render() {
    let tile_count = u32(atomicLoad(&parameters.final_index));
    let vertex_count = terrain_view.vertices_per_tile * tile_count;

    indirect_buffer.workgroup_count = vec3<u32>(vertex_count, 1u, 0u);
}