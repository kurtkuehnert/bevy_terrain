#define_import_path bevy_terrain::config

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

