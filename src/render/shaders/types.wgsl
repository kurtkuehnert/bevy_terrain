#define_import_path bevy_terrain::types

struct TerrainConfig {
    lod_count: u32,
    min_height: f32,
    max_height: f32,
}

struct TerrainViewConfig {
    view_local_position: vec3<f32>,
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
    side: u32,
    lod: u32,
    xy: vec2<u32>,
}

struct Parameters {
    tile_count: u32,
    counter: i32,
    child_index: atomic<i32>,
    final_index: atomic<i32>,
}

struct UVCoordinate {
    side: u32,
    uv: vec2<f32>,
}

struct Blend {
    lod: u32,
    ratio: f32,
}

struct QuadtreeEntry {
    atlas_index: u32,
    atlas_lod: u32,
}

struct LookupInfo {
    coordinate: UVCoordinate,
    view_distance: f32,
    lod: u32,
    blend_ratio: f32,
    ddx: vec2<f32>,
    ddy: vec2<f32>,
}

// A lookup of a node inside the node atlas based on the view of a quadtree.
struct NodeLookup {
    index: u32,
    lod: u32,
    coordinate: vec2<f32>,
    ddx: vec2<f32>,
    ddy: vec2<f32>,
}

struct AttachmentConfig {
    size: f32,
    scale: f32,
    offset: f32,
    _padding: u32,
}
