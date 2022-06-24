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


