#define_import_path bevy_terrain::vertex

#import bevy_terrain::types::{Tile, Blend}
#import bevy_terrain::bindings::{view_config, tiles}
#import bevy_terrain::functions::{lookup_node, compute_coordinate, compute_local_position, compute_relative_coordinate, compute_relative_position, compute_grid_offset, compute_morph, compute_blend, local_to_world_position, world_to_clip_position}
#import bevy_terrain::attachments::sample_height
#import bevy_pbr::mesh_view_bindings::view

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position)     clip_position: vec4<f32>,
    @location(0)           tile_index: u32,
    @location(1)           offset: vec2<f32>,
    @location(2)           view_distance: f32,
    @location(3)           world_normal: vec3<f32>,
    @location(4)           world_position: vec4<f32>,
    @location(5)           debug_color: vec4<f32>,
}

struct VertexInfo {
    tile_index: u32,
    tile: Tile,
    grid_offset: vec2<f32>,
    offset: vec2<f32>, // Todo: find better name
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    view_distance: f32,
}

fn setup_vertex_info(info: ptr<function, VertexInfo>, input: VertexInput) {
    let tile_index = input.vertex_index / view_config.vertices_per_tile;
    let grid_index = input.vertex_index % view_config.vertices_per_tile;

    let grid_offset = compute_grid_offset(grid_index);

    (*info).tile_index  = tile_index;
    (*info).tile        = tiles[tile_index];
    (*info).grid_offset = grid_offset;
    (*info).offset      = grid_offset;

    var coordinate     = compute_coordinate((*info).tile, (*info).grid_offset);
    var local_position = compute_local_position(coordinate);
    var world_position = local_to_world_position(local_position);
    let view_distance  = distance(world_position + view_config.approximate_height * local_position, view.world_position);

#ifdef MORPH
    var morph      = compute_morph(view_distance, (*info).tile.lod, (*info).grid_offset);
    coordinate     = compute_coordinate((*info).tile, morph.offset);
    local_position = compute_local_position(coordinate);
    world_position = local_to_world_position(local_position);

    (*info).offset = morph.offset;
#endif

    (*info).world_normal   = local_position;
    (*info).world_position = world_position;
    (*info).view_distance  = view_distance;
}

fn high_precision(info: ptr<function, VertexInfo>) {
    if ((*info).view_distance < view_config.precision_threshold_distance) {
        var relative_coordinate = compute_relative_coordinate((*info).tile, (*info).grid_offset);
        var relative_position   = compute_relative_position(relative_coordinate);
        let view_distance       = length(relative_position + view_config.approximate_height * (*info).world_normal);

    #ifdef MORPH
        let morph           = compute_morph(view_distance, (*info).tile.lod, (*info).grid_offset);
        relative_coordinate = compute_relative_coordinate((*info).tile, morph.offset);
        relative_position   = compute_relative_position(relative_coordinate);

        (*info).offset = morph.offset;
    #endif

        (*info).world_position = view.world_position + relative_position;
        (*info).view_distance  = view_distance;
    }
}

fn apply_height(info: ptr<function, VertexInfo>, blend: Blend) {
    let lookup = lookup_node((*info).tile, (*info).offset, blend, 0u);

    let height = sample_height(lookup);

    let world_position = (*info).world_position + height * (*info).world_normal;

    (*info).world_position = world_position;
    (*info).view_distance  = distance(world_position, view.world_position);
}
