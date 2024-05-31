#import bevy_terrain::types::NodeLookup
#import bevy_terrain::bindings::config
#import bevy_terrain::functions::{vertex_coordinate, lookup_node}
#import bevy_terrain::attachments::{sample_height_grad, sample_normal_grad}
#import bevy_terrain::vertex::{VertexInput, VertexOutput}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_lookup_info, fragment_output}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

#import bevy_terrain::bindings::{view_config, tiles, model_view_approximation}
#import bevy_terrain::types::{Blend, UVCoordinate, LookupInfo, Tile}
#import bevy_terrain::functions::{tile_size, coordinate_from_local_position, grid_offset, tile_coordinate, compute_morph, local_position_from_coordinate, compute_blend, quadtree_lod, local_to_world_position, world_to_clip_position}
#import bevy_terrain::attachments::sample_height
#import bevy_terrain::debug::{show_tiles, index_color}
#import bevy_pbr::mesh_view_bindings::view

@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient_sampler: sampler;
@group(3) @binding(2)
var<uniform> super_elevation: f32;

fn sample_color_grad(lookup: NodeLookup) -> vec4<f32> {
    let height = sample_height_grad(lookup);

    var color: vec4<f32>;

    if (height < 0.0) {
        color = textureSampleLevel(gradient, gradient_sampler, mix(0.0, 0.075, pow(height / config.min_height, 0.25)), 0.0);
    }
    else {
        color = textureSampleLevel(gradient, gradient_sampler, mix(0.09, 1.0, pow(height / config.max_height * 2.0, 1.0)), 0.0);
    }

    return color;
}

fn relative_coordinate(tile: Tile, vertex_offset: vec2<f32>) -> UVCoordinate {
    let side = model_view_approximation.sides[tile.side];

    let lod_difference = tile.lod - u32(model_view_approximation.origin_lod);
    let origin_xy = vec2<i32>(side.origin_xy.x << lod_difference, side.origin_xy.y << lod_difference);
    let tile_offset = vec2<i32>(tile.xy) - origin_xy;
    let relative_st = (vec2<f32>(tile_offset) + vertex_offset) * tile_size(tile.lod) + side.delta_relative_st;

    return UVCoordinate(tile.side, relative_st);
}

fn approximate_relative_position(relative_coordinate: UVCoordinate) -> vec3<f32> {
    let params = model_view_approximation.sides[relative_coordinate.side];

    let s = relative_coordinate.uv.x;
    let t = relative_coordinate.uv.y;
    let c = params.c;
    let c_s = params.c_s;
    let c_t = params.c_t;
    let c_ss = params.c_ss;
    let c_st = params.c_st;
    let c_tt = params.c_tt;

    return c + c_s * s + c_t * t + c_ss * s * s + c_st * s * t + c_tt * t * t;
}

fn show_approximation_origin(coordinate: UVCoordinate) -> vec4<f32> {
    let origin_lod = u32(model_view_approximation.origin_lod);
    let side = model_view_approximation.sides[coordinate.side];

    var color = vec4<f32>(0.0);

    let origin_tile = Tile(coordinate.side, origin_lod, vec2<u32>(side.origin_xy));
    let origin_coordinate = vec2<f32>(side.origin_xy) * tile_size(origin_lod);
    let camera_coordinate = UVCoordinate(coordinate.side, origin_coordinate - side.delta_relative_st);

    if (distance(coordinate.uv, origin_coordinate) < 0.01) {
        color += vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if (distance(coordinate.uv, camera_coordinate.uv) < 0.001) {
        color += vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }

    return color;
}

fn show_error(tile: Tile, vertex_offset: vec2<f32>) -> vec4<f32> {
    let relative_coordinate = relative_coordinate(tile, vertex_offset);
    let relative_position = approximate_relative_position(relative_coordinate);

    var color = vec4<f32>(0.0);

    color = index_color(u32(log2(length(relative_position))));

    color = vec4<f32>(relative_coordinate.uv, 0.0, 1.0);
    color = vec4<f32>(length(relative_position) / 50.0, 0.0, 0.0, 1.0);

    return color;
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let tile_index = input.vertex_index / view_config.vertices_per_tile;
    let grid_index = input.vertex_index % view_config.vertices_per_tile;

    let tile        = tiles[tile_index];
    let grid_offset = grid_offset(grid_index);

    let approximate_coordinate     = tile_coordinate(tile, vec2<f32>(grid_offset) / view_config.grid_size);
    let approximate_local_position = local_position_from_coordinate(approximate_coordinate, view_config.approximate_height);
    var approximate_distance       = distance(approximate_local_position, view_config.view_local_position);

#ifdef MORPH
    var morph_ratio  = compute_morph(approximate_distance, tile.lod);
    var morph_offset = mix(vec2<f32>(grid_offset), vec2<f32>(grid_offset & vec2<u32>(4294967294u)), morph_ratio);
    let coordinate   = tile_coordinate(tile, morph_offset / view_config.grid_size);
#else
    let coordinate   = approximate_coordinate;
#endif

    let local_position = local_position_from_coordinate(coordinate, 0.0);
    let world_normal = normalize(local_position);

    var view_distance = distance(view.world_position, local_to_world_position(local_position).xyz);

    var world_position = vec3<f32>(0.0);

    #ifdef TEST1
    let threshold_distance = 50000.0;
    #else
    let threshold_distance = 0.0;
    #endif

    if (view_distance < threshold_distance) {
        let offset              = vec2<f32>(grid_offset) / view_config.grid_size;
        var relative_coordinate = relative_coordinate(tile, offset);
        var relative_position   = approximate_relative_position(relative_coordinate);
        approximate_distance    = length(relative_position) / 6371000.0;

    #ifdef MORPH
        morph_ratio  = compute_morph(approximate_distance, tile.lod);
        morph_offset = mix(vec2<f32>(grid_offset), vec2<f32>(grid_offset & vec2<u32>(4294967294u)), morph_ratio) / view_config.grid_size;
        relative_coordinate = relative_coordinate(tile, morph_offset);
    #endif

        relative_position = approximate_relative_position(relative_coordinate);
        world_position = view.world_position + relative_position;
    } else {
        world_position = local_to_world_position(local_position);
    }

    // Todo: apply height to world_position
    let height = 0.0;
    world_position       += height * world_normal;
    view_distance         = distance(world_position, view.world_position) / 6371000.0;
    let fragment_position = world_to_clip_position(world_position);


#ifdef QUADTREE_LOD
    let blend = Blend(quadtree_lod(coordinate), 0.0);
#else
    let blend = compute_blend(approximate_distance);
#endif

    var output: VertexOutput;



    output.side              = coordinate.side;
    output.uv                = coordinate.uv;
    output.view_distance     = view_distance;
    output.world_normal      = world_normal;
    output.world_position    = vec4<f32>(world_position, 1.0);
    output.fragment_position = fragment_position;

#ifdef SHOW_TILES
    output.debug_color       = show_tiles(view_distance, input.vertex_index);
#endif

    // let w1 = local_to_world_position(local_position);
    // let w2 = view.world_position + relative_position;
//
    // var error = 0.0;
    // error = distance(w1, w2)/ length(w1);
    // error *= 5000.0;
//
    // // output.debug_color = show_error(tile, vec2<f32>(grid_offset) / view_config.grid_size);
    // output.debug_color = vec4<f32>(error, 0.0, 0.0, 1.0);

    return output;
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    let info = fragment_lookup_info(input);

    let lookup = lookup_node(info, 0u);
    var normal = sample_normal_grad(lookup, input.world_normal, input.side);
    var color  = sample_color_grad(lookup);

    if (info.blend_ratio > 0.0) {
        let lookup2 = lookup_node(info, 1u);
        normal      = mix(normal, sample_normal_grad(lookup2, input.world_normal, input.side), info.blend_ratio);
        color       = mix(color,  sample_color_grad(lookup2),                                  info.blend_ratio);
    }

    // color = show_approximation_origin(info.coordinate) * 5.0;

    return fragment_output(input, color, normal, lookup);
}
