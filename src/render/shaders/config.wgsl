#define_import_path bevy_terrain::config

struct TerrainConfig {
    lod_count: u32;
    patch_size: u32;
    chunk_size: u32;
    chunk_count: vec2<u32>;
    texture_size: u32;
    area_size: u32;
    area_count: vec2<u32>;
    terrain_size: vec2<u32>;
    vertices_per_row: u32;
    scale: f32;
    height: f32;
    node_atlas_size: u32;
};
