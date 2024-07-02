#define_import_path bevy_terrain::vertex

#import bevy_terrain::types::{Tile, Blend, NodeLookup}
#import bevy_terrain::bindings::{config, view_config, tiles}
#import bevy_terrain::functions::{lookup_node, compute_grid_offset, compute_local_position, compute_relative_position, compute_morph, compute_blend, normal_local_to_world, position_local_to_world}
#import bevy_terrain::attachments::{sample_height}
#import bevy_terrain::debug::{show_tiles}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::view_transformations::position_world_to_clip

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       tile_index: u32,
    @location(1)       offset: vec2<f32>,
    @location(2)       view_distance: f32,
    @location(3)       world_normal: vec3<f32>,
    @location(4)       world_position: vec4<f32>,
}

struct VertexInfo {
    tile_index: u32,
    tile: Tile,
    offset: vec2<f32>,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    blend: Blend,
}

fn vertex_info(input: VertexInput) -> VertexInfo {
    let tile_index = input.vertex_index / view_config.vertices_per_tile;
    let grid_index = input.vertex_index % view_config.vertices_per_tile;

    var info: VertexInfo;
    info.tile_index  = tile_index;

    let tile           = tiles[tile_index];
    let grid_offset    = compute_grid_offset(grid_index);
    var local_position = compute_local_position(tile, grid_offset);
    let world_position = position_local_to_world(local_position);
    let world_normal   = normal_local_to_world(local_position);
    var view_distance  = distance(world_position + view_config.approximate_height * world_normal, view.world_position);

#ifdef TEST1
    let high_precision = view_distance < view_config.precision_threshold_distance;
#else
    let high_precision = false;
#endif

    if (high_precision) {
        var relative_position = compute_relative_position(tile, grid_offset);
        view_distance         = length(relative_position + view_config.approximate_height * world_normal);

        let morph             = compute_morph(view_distance, tile.lod, grid_offset);
        relative_position     = compute_relative_position(tile, morph.offset);

        info.offset           = morph.offset;
        info.world_position   = view.world_position + relative_position;
        info.world_normal     = world_normal;
    } else {
         let morph            = compute_morph(view_distance, tile.lod, grid_offset);
         local_position       = compute_local_position(tile, morph.offset);

         info.offset          = morph.offset;
         info.world_position  = position_local_to_world(local_position);
         info.world_normal    = normal_local_to_world(local_position);
    }

    info.tile  = tile;
    info.blend = compute_blend(view_distance);

    return info;
}

fn vertex_lookup_node(info: ptr<function, VertexInfo>, lod_offset: u32) -> NodeLookup {
    return lookup_node((*info).tile, (*info).offset, vec2<f32>(0.0), vec2<f32>(0.0), (*info).blend, lod_offset);
}

fn vertex_output(info: ptr<function, VertexInfo>, height: f32) -> VertexOutput {
    let world_position = (*info).world_position + height * (*info).world_normal;

    var output: VertexOutput;
    output.tile_index     = (*info).tile_index;
    output.offset         = (*info).offset;
    output.view_distance  = distance(world_position, view.world_position);
    output.world_normal   = (*info).world_normal;
    output.world_position = vec4<f32>(world_position, 1.0);
    output.clip_position  = position_world_to_clip(world_position);
    return output;
}

fn vertex_default(input: VertexInput) -> VertexOutput {
    var info = vertex_info(input);

    let lookup = vertex_lookup_node(&info, 0u);
    var height = sample_height(lookup);

    if (info.blend.ratio > 0.0) {
        let lookup2 = vertex_lookup_node(&info, 1u);
        height      = mix(height, sample_height(lookup2), info.blend.ratio);
    }

    return vertex_output(&info, height);
}
