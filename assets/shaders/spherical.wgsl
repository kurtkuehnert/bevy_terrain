#import bevy_terrain::types::NodeLookup
#import bevy_terrain::bindings::config
#import bevy_terrain::functions::{vertex_coordinate, lookup_node}
#import bevy_terrain::attachments::{sample_height_grad, sample_normal_grad}
#import bevy_terrain::vertex::{VertexInput, VertexOutput}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_lookup_info, fragment_output}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

#import bevy_terrain::bindings::{view_config, tiles, model_view_approximation}
#import bevy_terrain::types::{Blend, Coordinate, LookupInfo, Tile, Morph}
#import bevy_terrain::functions::{tile_size, compute_coordinate, compute_local_position, compute_relative_coordinate, compute_relative_position, compute_grid_offset, compute_morph, compute_blend, local_to_world_position, world_to_clip_position}
#import bevy_terrain::attachments::sample_height
#import bevy_terrain::debug::{show_tiles, index_color}
#import bevy_pbr::mesh_view_bindings::view

struct VertexInfo {
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    view_distance: f32,
}

fn default_precision(info: ptr<function, VertexInfo>, tile: Tile, grid_offset: vec2<f32>) {
    let approximate_coordinate     = compute_coordinate(tile, grid_offset);
    let approximate_local_position = compute_local_position(approximate_coordinate);
    let approximate_world_position = local_to_world_position(approximate_local_position) + view_config.approximate_height * approximate_local_position;
    var approximate_view_distance  = distance(approximate_world_position, view.world_position);

#ifdef MORPH
    var morph      = compute_morph(approximate_view_distance, tile.lod, grid_offset);
    let coordinate = compute_coordinate(tile, morph.offset);
#else
    let coordinate = approximate_coordinate;
#endif

    let local_position = compute_local_position(coordinate);
    (*info).world_position = local_to_world_position(local_position);
    (*info).world_normal   = normalize(local_position);
    (*info).view_distance = approximate_view_distance;
}

fn high_precision(info: ptr<function, VertexInfo>, tile: Tile, grid_offset: vec2<f32>) {
    #ifdef TEST1
    let threshold_distance = 50000.0;
    #else
    let threshold_distance = 0.0;
    #endif

    if ((*info).view_distance < threshold_distance) {
        let approximate_relative_coordinate = compute_relative_coordinate(tile, grid_offset);
        let approximate_relative_position   = compute_relative_position(approximate_relative_coordinate);
        let approximate_view_distance           = length(approximate_relative_position);

    #ifdef MORPH
        let morph = compute_morph(approximate_view_distance, tile.lod, grid_offset);
        let relative_coordinate = compute_relative_coordinate(tile, morph.offset);
    #else
        let relative_coordinate = approximate_relative_coordinate;
    #endif

        let relative_position  = compute_relative_position(relative_coordinate);
        (*info).world_position = view.world_position + relative_position;
        (*info).view_distance  = approximate_view_distance;
    }
}

fn apply_height(info: ptr<function, VertexInfo>, blend: Blend) {
    // Todo: apply height to world_position
    let height = 0.0;

    let world_position = (*info).world_position + height * (*info).world_normal;

    (*info).world_position = world_position;
    (*info).view_distance  = distance(world_position, view.world_position);
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let tile_index = input.vertex_index / view_config.vertices_per_tile;
    let grid_index = input.vertex_index % view_config.vertices_per_tile;

    let tile        = tiles[tile_index];
    let grid_offset = compute_grid_offset(grid_index);

    var info: VertexInfo;

    default_precision(&info, tile, grid_offset);
    high_precision(&info, tile, grid_offset);

    let blend = compute_blend(info.view_distance);

    // terrain data sample hook
    apply_height(&info, blend);

    let clip_position  = world_to_clip_position(info.world_position);

    var output: VertexOutput;
    output.side              = tile.side;
    output.uv                = grid_offset;
    output.view_distance     = info.view_distance;
    output.world_normal      = info.world_normal;
    output.world_position    = vec4<f32>(info.world_position, 1.0);
    output.clip_position     = clip_position;

#ifdef SHOW_TILES
    output.debug_color       = show_tiles(info.view_distance, input.vertex_index);
#endif

    return output;
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    let info = fragment_lookup_info(input);

    let lookup = lookup_node(info, 0u);
    var normal = sample_normal_grad(lookup, input.world_normal, input.side);

    if (info.blend_ratio > 0.0) {
        let lookup2 = lookup_node(info, 1u);
        normal      = mix(normal, sample_normal_grad(lookup2, input.world_normal, input.side), info.blend_ratio);
    }

    // color = show_approximation_origin(info.coordinate) * 5.0;

    let color = vec4<f32>(0.5);

    return fragment_output(input, color, normal, lookup);
}
