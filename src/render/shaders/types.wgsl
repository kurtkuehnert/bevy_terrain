#define_import_path bevy_terrain::types

struct Mesh { flags: u32 }; let mesh = Mesh(0u); // hack for the pbr shaders

struct TerrainViewConfig {
    height_under_viewer: f32,

    node_count: u32,

    tile_count: u32,
    refinement_count: u32,
    view_distance: f32,
    tile_scale: f32,

    morph_blend: f32,
    vertex_blend: f32,
    fragment_blend: f32,
}

struct Tile {
    coords: vec2<u32>,
    size: u32,
    counts: u32,
    parent_counts: u32,
    padding: u32,
}

struct TileList {
    counts: array<vec2<u32>, 4>,
    data: array<Tile>,
}

fn calc_tile_count(lod: u32) -> u32 {
    return (lod + 1u) << 1u; // 2, 4, 6, 8, ...
}