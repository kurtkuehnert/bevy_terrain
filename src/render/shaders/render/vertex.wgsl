#define_import_path bevy_terrain::vertex

#import bevy_terrain::bindings::{view_config, tiles}
#import bevy_terrain::types::{Blend, UVCoordinate, LookupInfo}
#import bevy_terrain::functions::{grid_offset, tile_coordinate, compute_morph, local_position_from_coordinate, compute_blend, quadtree_lod, lookup_node, local_to_world_position, world_to_clip_position}
#import bevy_terrain::attachments::sample_height
#import bevy_terrain::debug::show_tiles

struct VertexInput {
    @builtin(vertex_index)   vertex_index: u32,
}

struct VertexOutput {
    @builtin(position)       fragment_position: vec4<f32>,
    @location(0)             side: u32,
    @location(1)             uv: vec2<f32>,
    @location(2)             view_distance: f32,
    @location(3)             world_normal: vec3<f32>,
    @location(4)             world_position: vec4<f32>,
    @location(5)             debug_color: vec4<f32>,
}

fn vertex_lookup_info(input: VertexInput) -> LookupInfo {
    let tile_index = input.vertex_index / view_config.vertices_per_tile;
    let grid_index = input.vertex_index % view_config.vertices_per_tile;

    let tile        = tiles[tile_index];
    let grid_offset = grid_offset(grid_index);

    let approximate_coordinate  = tile_coordinate(tile, vec2<f32>(grid_offset) / view_config.grid_size);
    let approximate_position    = local_position_from_coordinate(approximate_coordinate, view_config.approximate_height);
    let approximate_distance    = distance(approximate_position, view_config.view_local_position);

#ifdef MORPH
    let morph_ratio  = compute_morph(approximate_distance, tile.lod);
    let morph_offset = mix(vec2<f32>(grid_offset), vec2<f32>(grid_offset & vec2<u32>(4294967294u)), morph_ratio);
    let coordinate   = tile_coordinate(tile, morph_offset / view_config.grid_size);
#else
    let coordinate   = approximate_coordinate;
#endif

#ifdef QUADTREE_LOD
    let blend = Blend(quadtree_lod(coordinate), 0.0);
#else
    let blend = compute_blend(approximate_distance);
#endif

    return LookupInfo(coordinate, approximate_distance, blend.lod, blend.ratio, vec2<f32>(0.0), vec2<f32>(0.0));
}

fn vertex_output(input: VertexInput, info: LookupInfo, height: f32) -> VertexOutput {
    var output: VertexOutput;

    let local_position = local_position_from_coordinate(info.coordinate, height);
    let view_distance  = distance(local_position, view_config.view_local_position);

    output.side              = info.coordinate.side;
    output.uv                = info.coordinate.uv;
    output.view_distance     = view_distance;
    output.world_normal      = normalize(local_position);
    output.world_position    = local_to_world_position(local_position);
    output.fragment_position = world_to_clip_position(output.world_position);


#ifdef SHOW_TILES
    output.debug_color       = show_tiles(info.view_distance, input.vertex_index);
#endif

    return output;
}

fn default_vertex(input: VertexInput) -> VertexOutput {
    let info = vertex_lookup_info(input);

    let lookup = lookup_node(info, 0u);
    var height = sample_height(lookup);

    if (info.blend_ratio > 0.0) {
        let lookup2 = lookup_node(info, 1u);
        height      = mix(height, sample_height(lookup2), info.blend_ratio);
    }

     return vertex_output(input, info, height);
}
