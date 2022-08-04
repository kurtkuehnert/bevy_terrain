#define_import_path bevy_terrain::vertex

fn height_vertex(atlas_index: i32, atlas_coords: vec2<f32>) -> f32 {
    let height_coords = atlas_coords * config.height_scale + config.height_offset;
    return config.height * textureSampleLevel(height_atlas, terrain_sampler, height_coords, atlas_index, 0.0).x;
}

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    var tile_lod = 0u;
    for (; tile_lod < 4u; tile_lod = tile_lod + 1u) {
        if (vertex.index < tiles.counts[tile_lod].y) {
            break;
        }
    }

    let tile_size = calc_tile_count(tile_lod);
    let vertices_per_row = (tile_size + 2u) << 1u;
    let vertices_per_tile = vertices_per_row * tile_size;

    let tile_index  = (vertex.index - tiles.counts[tile_lod].x) / vertices_per_tile + tile_lod * 100000u;
    let vertex_index = (vertex.index - tiles.counts[tile_lod].x) % vertices_per_tile;

    let tile = tiles.data[tile_index];
    let local_position = calculate_position(vertex_index, tile, vertices_per_row, tile_size);

    let world_position = vec3<f32>(local_position.x, view_config.height_under_viewer, local_position.y);
    let blend = calculate_blend(world_position, view_config.vertex_blend);

    let lookup = atlas_lookup(blend.log_distance, local_position);
    var height = height_vertex(lookup.atlas_index, lookup.atlas_coords);

    if (blend.ratio < 1.0) {
        let lookup2 = atlas_lookup(blend.log_distance + 1.0, local_position);
        var height2 = height_vertex(lookup2.atlas_index, lookup2.atlas_coords);
        height = mix(height2, height, blend.ratio);
    }

    var output = vertex_output(local_position, height);

#ifdef SHOW_TILES
    output.color = show_tiles(tile, local_position, tile_lod);
#endif

    return output;
}