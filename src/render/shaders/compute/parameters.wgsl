#define_import_path bevy_terrain::parameters

struct Parameters {
    refinement_count: u32,
    counter: i32,
    child_index: atomic<i32>,
    final_index1: atomic<i32>,
    final_index2: atomic<i32>,
    final_index3: atomic<i32>,
    final_index4: atomic<i32>,
    // final_indices:  array<atomic<i32>, 4>,
}

struct TileInfo {
    coords: vec2<u32>,
    size: u32,
}

struct TemporaryTileList {
    data: array<TileInfo>,
}