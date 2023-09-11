#define_import_path bevy_terrain::node
#import bevy_terrain::types TerrainConfig, TerrainViewConfig,Tile, TileList
 
 
 #import bevy_terrain::uniforms view_config, quadtree , tiles, config 
 
#import bevy_pbr::mesh_view_bindings       view 
 
 

// A lookup of a node inside the node atlas based on the view of a quadtree.
struct NodeLookup {
    atlas_lod: u32,
    atlas_index: i32,
    atlas_coords: vec2<f32>,
}

fn approximate_world_position(local_position: vec2<f32> ) -> vec4<f32> {
    return vec4<f32>(local_position.x, view_config.approximate_height, local_position.y, 1.0);
}

fn node_size(lod: u32 ) -> f32 {
    return f32(config.leaf_node_size * (1u << lod));
}

// Looks up the best availale node in the node atlas from the viewers point of view.
// This is done by sampling the viewers quadtree at the caluclated coordinate.
fn lookup_node(lod: u32, local_position: vec2<f32>  ) -> NodeLookup {
#ifdef SHOW_NODES
    var quadtree_lod = 0u;
    for (; quadtree_lod < config.lod_count; quadtree_lod = quadtree_lod + 1u) {
        let coordinate = local_position / node_size(quadtree_lod);
        let grid_coordinate = floor(view.world_position.xz / node_size(quadtree_lod) + 0.5 - f32(view_config.node_count >> 1u));

        let grid = step(grid_coordinate, coordinate) * (1.0 - step(grid_coordinate + f32(view_config.node_count), coordinate));

        if (grid.x * grid.y == 1.0) {
            break;
        }
    }
#else
    let quadtree_lod = min(lod, config.lod_count - 1u);
#endif

    let quadtree_coords = vec2<i32>((local_position / node_size(quadtree_lod)) % f32(view_config.node_count));
    let lookup = textureLoad(quadtree, quadtree_coords, i32(quadtree_lod), 0);

    let atlas_index = i32(lookup.x);
    let atlas_lod   = lookup.y;
    let atlas_coords = (local_position / node_size(atlas_lod)) % 1.0;

    return NodeLookup(atlas_lod, atlas_index, atlas_coords);
}
