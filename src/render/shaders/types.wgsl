#define_import_path bevy_terrain::types

struct TerrainConfig {
    lod_count: u32,
    min_height: f32,
    max_height: f32,
    leaf_node_count: f32,
}

struct TerrainViewConfig {
    approximate_height: f32,
    quadtree_size: u32,
    tile_count: u32,
    refinement_count: u32,
    grid_size: f32,
    vertices_per_row: u32,
    vertices_per_tile: u32,
    morph_distance: f32,
    blend_distance: f32,
    morph_range: f32,
    blend_range: f32,
}

struct Tile {
    uv: vec2<f32>, // [0..1]
    size: f32, // [0..1]
    side: u32, // [0..6]
}

struct TileList {
    data: array<Tile>,
}

struct Parameters {
    tile_count: u32,
    counter: i32,
    child_index: atomic<i32>,
    final_index: atomic<i32>,
}

struct S2Coordinate {
    side: u32,
    st: vec2<f32>,
}

struct Morph {
    ratio: f32,
}

struct Blend {
    lod: u32,
    ratio: f32,
}

// A lookup of a node inside the node atlas based on the view of a quadtree.
struct NodeLookup {
    atlas_index: u32,
    atlas_lod: u32,
    atlas_coordinate: vec2<f32>,
}
