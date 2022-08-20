#define_import_path bevy_terrain::atlas

// A lookup in the node atlas based on the view of a quadtree.
struct AtlasLookup {
    lod: u32,
    atlas_index: i32,
    atlas_coords: vec2<f32>,
}

fn node_size(lod: u32) -> f32 {
    return f32(config.chunk_size * (1u << lod));
}

// Looks up the best availale node in the node atlas from the viewers point of view.
// This is done by sampling the viewers quadtree at the caluclated location.
fn atlas_lookup(log_distance: f32, local_position: vec2<f32>) -> AtlasLookup {
    let lod = clamp(u32(log_distance), 0u, config.lod_count - 1u);

// #ifndef CIRCULAR_LOD
//     for (var lod = 0u; lod < config.lod_count; lod = lod + 1u) {
//         let coordinate = local_position / node_size(lod);
//         let grid_coordinate = floor(view.world_position.xz / node_size(lod) + 0.5 - f32(view_config.node_count >> 1u));
//
//         let grid = step(grid_coordinate, coordinate) * (1.0 - step(grid_coordinate + f32(view_config.node_count), coordinate));
//
//         if (grid.x * grid.y == 1.0) {
//             break;
//         }
//     }
// #endif

    let map_coords = vec2<i32>((local_position / node_size(lod)) % f32(view_config.node_count));
    let lookup = textureLoad(quadtree, map_coords, i32(lod), 0);

    let atlas_index = i32(lookup.x);
    let atlas_lod   = lookup.y;
    let atlas_coords = (local_position / node_size(atlas_lod)) % 1.0;

    return AtlasLookup(atlas_lod, atlas_index, atlas_coords);
}

fn approximate_world_position(local_position: vec2<f32>) -> vec3<f32> {
    return vec3<f32>(local_position.x, view_config.height_under_viewer, local_position.y);
}