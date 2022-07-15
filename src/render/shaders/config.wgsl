#define_import_path bevy_terrain::config

struct TerrainConfig {
    lod_count: u32,
    height: f32,
    chunk_size: u32,
}

struct TerrainViewConfig {
    height_under_viewer: f32,

    node_count: u32,

    terrain_size: u32,
    tile_count: u32,
    refinement_count: u32,
    view_distance: f32,
    tile_scale: f32,
}
