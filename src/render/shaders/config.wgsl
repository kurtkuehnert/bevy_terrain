#define_import_path bevy_terrain::config

struct TerrainConfig {
    lod_count: u32;
    chunk_size: u32;
    patch_size: u32;
    node_count: u32;
    vertices_per_row: u32;
    vertices_per_patch: u32;
    view_distance: f32;
    scale: f32;
    height: f32;
};




