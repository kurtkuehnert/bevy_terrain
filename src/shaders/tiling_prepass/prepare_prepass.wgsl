#import bevy_terrain::types::{TileCoordinate, Blend}
#import bevy_terrain::bindings::{terrain_view, approximate_height_write, temporary_tiles, state, indirect_buffer}
#import bevy_terrain::functions::{compute_view_coordinate, lookup_tile, compute_world_coordinate, apply_height}
#import bevy_terrain::attachments::{sample_height, sample_attachment0_gather0}

@compute @workgroup_size(1, 1, 1)
fn prepare_root() {
    state.counter = -1;
    atomicStore(&state.child_index, i32(terrain_view.tile_count - 1u));
    atomicStore(&state.final_index, 0);

#ifdef SPHERICAL
    state.tile_count = 6u;

    // Todo: consider culling the entire back face (opposite of viewer)
    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        temporary_tiles[i] = TileCoordinate(i, 0u, vec2<u32>(0u));
    }
#else
    state.tile_count = 1u;

    temporary_tiles[0] = TileCoordinate(0u, 0u, vec2<u32>(0u));
#endif

    indirect_buffer.workgroup_count = vec3<u32>(1u, 1u, 1u);

    // compute approximate height
    let coordinate = compute_view_coordinate(terrain_view.view_face, terrain_view.view_lod);
    let tile       = lookup_tile(coordinate, Blend(coordinate.lod, 0.0), 0u);
    let raw_height = sample_attachment0_gather0(tile);
    let mask       = bitcast<vec4<u32>>(raw_height) & vec4<u32>(1);

    if (all(mask != vec4<u32>(0))) {
        approximate_height_write = sample_height(tile);
    }

    // Todo: this should use high precision as well
    let world_coordinate = compute_world_coordinate(coordinate);
    let distance = dot(world_coordinate.normal, terrain_view.view_world_position) -
                   dot(world_coordinate.normal, apply_height(world_coordinate, approximate_height_write));

    if (distance < 0.0) {
        state.tile_count = 0u;
    }
}

@compute @workgroup_size(1, 1, 1)
fn prepare_next() {
    if (state.counter == 1) {
        state.tile_count = u32(atomicExchange(&state.child_index, i32(terrain_view.tile_count - 1u)));
    }
    else {
        state.tile_count = terrain_view.tile_count - 1u - u32(atomicExchange(&state.child_index, 0));
    }

    state.counter = -state.counter;
    indirect_buffer.workgroup_count.x = (state.tile_count + 63u) / 64u;
}

@compute @workgroup_size(1, 1, 1)
fn prepare_render() {
    let tile_count = u32(atomicLoad(&state.final_index));
    let vertex_count = terrain_view.vertices_per_tile * tile_count;

    indirect_buffer.workgroup_count = vec3<u32>(vertex_count, 1u, 0u);
}