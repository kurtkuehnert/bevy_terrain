#define_import_path bevy_terrain::atlas

// Todo: precompute the node sizes?
fn node_size(lod: u32) -> f32 {
    return f32(config.chunk_size * (1u << lod));
}

struct AtlasLookup {
    lod: u32;
    atlas_index: i32;
    atlas_coords: vec2<f32>;
};

fn atlas_lookup(log_distance: f32, world_position: vec2<f32>) -> AtlasLookup {
    let lod = clamp(u32(log_distance), 0u, config.lod_count - 1u);

#ifndef CIRCULAR_LOD
    for (var lod = 0u; lod < config.lod_count; lod = lod + 1u) {
        let coordinate = world_position / node_size(lod);
        let grid_coordinate = floor(view.world_position.xz / node_size(lod) - 0.5 * f32(config.node_count - 1u));

        let grid = step(grid_coordinate, coordinate) * (1.0 - step(grid_coordinate + f32(config.node_count), coordinate));

        if (grid.x * grid.y == 1.0) {
            break;
        }
    }
#endif

    let map_coords = vec2<i32>((world_position / node_size(lod)) % f32(config.node_count));
    let lookup = textureLoad(quadtree, map_coords, i32(lod), 0);

    let atlas_lod = lookup.z;
    let atlas_index =  i32((lookup.x << 8u) + lookup.y);
    let atlas_coords = (world_position / node_size(atlas_lod)) % 1.0;

    return AtlasLookup(atlas_lod, atlas_index, atlas_coords);
}

fn calculate_position(vertex_index: u32, patch: Patch) -> vec2<f32> {
    // use first and last index twice, to form degenerate triangles
    // Todo: documentation
    let row_index = clamp(vertex_index % config.vertices_per_row, 1u, config.vertices_per_row - 2u) - 1u;
    var vertex_position = vec2<u32>((row_index & 1u) + vertex_index / config.vertices_per_row, row_index >> 1u);

#ifndef MESH_MORPH
    // stitch the edges of the patches together
    if (vertex_position.x == 0u && (patch.stitch & 1u) != 0u) {
        vertex_position.y = vertex_position.y & 0xFFFEu; // mod 2
    }
    if (vertex_position.y == 0u && (patch.stitch & 2u) != 0u) {
        vertex_position.x = vertex_position.x & 0xFFFEu; // mod 2
    }
    if (vertex_position.x == config.patch_size && (patch.stitch & 4u) != 0u) {
        vertex_position.y = vertex_position.y + 1u & 0xFFFEu; // mod 2
    }
    if (vertex_position.y == config.patch_size && (patch.stitch & 8u) != 0u) {
        vertex_position.x = vertex_position.x + 1u & 0xFFFEu; // mod 2
    }
#endif

    var local_position = vec2<f32>((patch.coords + vertex_position) * patch.size);

#ifdef MESH_MORPH
    let viewer_distance = distance(local_position, view.world_position.xz);
    let morph_distance = f32(patch.size << 1u) * config.view_distance;
    let morph = clamp(1.0 - (1.0 - viewer_distance / morph_distance) / morph_blend, 0.0, 1.0);

    if (morph > 0.0) {
        let frac_part = ((vec2<f32>(vertex_position) * 0.5) % 1.0) * 2.0;
        local_position = local_position - frac_part * f32(patch.size) * morph;
    }
#endif

    return local_position;
}

fn calculate_normal(uv: vec2<f32>, atlas_index: i32, lod: u32) -> vec3<f32> {
    let left  = textureSampleLevel(height_atlas, filter_sampler, uv, atlas_index, 0.0, vec2<i32>(-1,  0)).x;
    let up    = textureSampleLevel(height_atlas, filter_sampler, uv, atlas_index, 0.0, vec2<i32>( 0, -1)).x;
    let right = textureSampleLevel(height_atlas, filter_sampler, uv, atlas_index, 0.0, vec2<i32>( 1,  0)).x;
    let down  = textureSampleLevel(height_atlas, filter_sampler, uv, atlas_index, 0.0, vec2<i32>( 0,  1)).x;

    return normalize(vec3<f32>(right - left, f32(2u << lod) / config.height, down - up));
}

fn height_vertex(atlas_index: i32, atlas_coords: vec2<f32>) -> f32 {
    let height_coords = atlas_coords * height_scale + height_offset;
    return config.height * textureSampleLevel(height_atlas, filter_sampler, height_coords, atlas_index, 0.0).x;
}


