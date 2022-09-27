#define_import_path bevy_terrain::vertex

fn height_vertex(atlas_index: i32, atlas_coords: vec2<f32>) -> f32 {
    let height_coords = atlas_coords * config.height_scale + config.height_offset;
    var height = textureSampleLevel(height_atlas, terrain_sampler, height_coords, atlas_index, 0.0).x;

#ifdef TEST3
    // still produces bugs for nodes of mip 1 and above (preprocessing)
    let gather = textureGather(0, height_atlas, terrain_sampler, height_coords, atlas_index);

    if (gather.x == 0.0 || gather.y == 0.0 || gather.z == 0.0 || gather.w == 0.0) {
        height = height / 0.0;
    }
#endif

    return config.height * height;
}

fn normal_vertex(atlas_index: i32, atlas_coords: vec2<f32>, lod: u32) -> vec3<f32> {
    let height_coords = atlas_coords * config.height_scale + config.height_offset;
    return calculate_normal(height_coords, atlas_index, lod);
}

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
#ifdef ADAPTIVE
    var tile_lod = 0u;
    for (; tile_lod < 4u; tile_lod = tile_lod + 1u) {
        if (vertex.index < tiles.counts[tile_lod].y) {
            break;
        }
    }

    let tile_size = calc_tile_count(tile_lod);
#else
    let tile_lod = 0u;
    let tile_size = 8u;
#endif

    let vertices_per_row = (tile_size + 2u) << 1u;
    let vertices_per_tile = vertices_per_row * tile_size;

    let tile_index = (vertex.index - tiles.counts[tile_lod].x) / vertices_per_tile + tile_lod * 100000u;
    let grid_index = (vertex.index - tiles.counts[tile_lod].x) % vertices_per_tile;

    let tile = tiles.data[tile_index];
    let local_position = calculate_position(grid_index, tile, vertices_per_row, tile_size);

    let world_position = approximate_world_position(local_position);
    let blend = calculate_blend(world_position, view_config.vertex_blend);

    let lookup = atlas_lookup(blend.lod, local_position);
    var height = height_vertex(lookup.atlas_index, lookup.atlas_coords);

#ifdef VERTEX_NORMAL
    var normal = normal_vertex(lookup.atlas_index, lookup.atlas_coords, lookup.lod);
#endif

    if (blend.ratio < 1.0) {
        let lookup2 = atlas_lookup(blend.lod + 1u, local_position);

        let height2 = height_vertex(lookup2.atlas_index, lookup2.atlas_coords);
        height = mix(height2, height, blend.ratio);

#ifdef VERTEX_NORMAL
        let normal2 = normal_vertex(lookup.atlas_index, lookup.atlas_coords, lookup.lod);
        normal = mix(normal2, normal, blend.ratio);
#endif
    }

    var output = vertex_output(local_position, height);

#ifdef VERTEX_NORMAL
    output.world_normal = normal;
#endif

#ifdef SHOW_TILES
    output.color = show_tiles(tile, local_position, tile_lod);
#endif

#ifdef MINMAX_ERROR
    let size = f32(tile.size) * view_config.tile_scale;
    let local_position = (vec2<f32>(tile.coords) + 0.5) * size;
    let lod = u32(ceil(log2(size))) + 1u;
    let minmax = minmax(local_position, size);

    output.color = vec4<f32>(0.0,
                             clamp((minmax.y - height) / size / 2.0, 0.0, 1.0),
                             clamp((height - minmax.x) / size / 2.0, 0.0, 1.0),
                             0.5);

    let tolerance = 0.00001;
    if (height < minmax.x - tolerance || height > minmax.y + tolerance || lod >= config.lod_count) {
        output.color = vec4<f32>(1.0, 0.0, 0.0, 0.5);
    }
#endif

    return output;
}
