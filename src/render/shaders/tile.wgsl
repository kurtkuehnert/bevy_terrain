#define_import_path bevy_terrain::tile

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