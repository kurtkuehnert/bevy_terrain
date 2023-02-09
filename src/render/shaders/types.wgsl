#define_import_path bevy_terrain::types

struct TerrainViewConfig {
    approximate_height: f32,
    node_count: u32,

    tile_count: u32,
    refinement_count: u32,
    tile_scale: f32,
    grid_size: f32,
    vertices_per_row: u32,
    vertices_per_tile: u32,
    morph_distance: f32,
    blend_distance: f32,
    morph_range: f32,
    blend_range: f32,
}

struct Tile {
    coords: vec2<u32>,
    size: u32,
}

struct TileList {
    data: array<Tile>,
}
