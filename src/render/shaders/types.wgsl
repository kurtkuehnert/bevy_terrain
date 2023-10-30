#define_import_path bevy_terrain::types

struct TerrainConfig {
    lod_count: u32,
    height: f32,
    nodes_per_side: f32,
    leaf_node_size: u32,
    terrain_size: f32,
    radius: f32,
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
    view_distance: f32,
    morph_range: f32,
    blend_range: f32,
}

struct Tile {
    uv: vec2<f32>, // [0..1]
    size: f32, // [0..1]
    side: u32, // [0..6]
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

struct S2Coordinate {
    side: u32,
    st: vec2<f32>,
}

// A lookup of a node inside the node atlas based on the view of a quadtree.
struct NodeLookup {
    atlas_index: u32,
    atlas_lod: u32,
    atlas_coordinate: vec2<f32>,
}

struct VertexInput {
    @builtin(instance_index) instance: u32,
    @builtin(vertex_index)   vertex_index: u32,
}

struct VertexOutput {
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec3<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}

struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec3<f32>,
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

