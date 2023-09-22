#define_import_path bevy_terrain::types

// Todo: split into TerrainConfig and AttachmentConfig
struct TerrainConfig {
    lod_count: u32,
    height: f32,
    leaf_node_size: u32,
    terrain_size: u32,

    height_size: f32,
    minmax_size: f32,
    _empty_a: u32,
    _empty_b: u32,
    height_scale: f32,
    minmax_scale: f32,
    _empty_c: u32,
    _empty_d: u32,
    height_offset: f32,
    minmax_offset: f32,
    _empty_e: u32,
    _empty_f: u32,
}

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

struct Parameters {
    tile_count: u32,
    counter: i32,
    child_index: atomic<i32>,
    final_index: atomic<i32>,
}

// A lookup of a node inside the node atlas based on the view of a quadtree.
struct NodeLookup {
    atlas_lod: u32,
    atlas_index: i32,
    atlas_coords: vec2<f32>,
}

struct VertexInput {
    @builtin(instance_index) instance: u32,
    @builtin(vertex_index)   vertex_index: u32,
}

struct VertexOutput {
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec2<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}

struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec2<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}

struct FragmentOutput {
    @location(0)             color: vec4<f32>
}

// The processed fragment consisting of the color and a flag whether or not to discard this fragment.
struct Fragment {
    color: vec4<f32>,
    do_discard: bool,
}

struct Blend {
    lod: u32,
    ratio: f32,
}

