#define_import_path bevy_terrain::vertex

#import bevy_terrain::types::{Blend, AtlasTile, Coordinate, TileCoordinate, WorldCoordinate}
#import bevy_terrain::bindings::{terrain, terrain_view, approximate_height, geometry_tiles}
#import bevy_terrain::functions::{compute_coordinate, compute_world_coordinate, apply_height, lookup_tile, compute_tile_uv, compute_world_coordinate_precise, compute_morph, compute_blend}
#import bevy_terrain::attachments::{sample_height, sample_attachment0_gather0}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::view_transformations::position_world_to_clip

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       tile_index: u32,
    @location(1)       coordinate_uv: vec2<f32>,
    @location(2)       world_position: vec4<f32>, // Todo: consider reconstructing position and normal in fragment shader
    @location(3)       world_normal: vec3<f32>,
}

struct VertexInfo {
    tile_index: u32,
    coordinate: Coordinate,
    world_coordinate: WorldCoordinate,
    blend: Blend,
}

fn vertex_info(input: VertexInput) -> VertexInfo {
    let approximate_coordinate       = compute_coordinate(input.vertex_index);
    var approximate_world_coordinate = compute_world_coordinate(approximate_coordinate);
    var approximate_view_distance    = distance(apply_height(approximate_world_coordinate, approximate_height), view.world_position);

    var info: VertexInfo;
    info.tile_index = input.vertex_index / terrain_view.vertices_per_tile;

#ifdef HIGH_PRECISION
    if (approximate_view_distance < terrain_view.precision_threshold_distance) {
        approximate_world_coordinate = compute_world_coordinate_precise(approximate_coordinate, approximate_world_coordinate.normal);
        approximate_view_distance    = distance(apply_height(approximate_world_coordinate, approximate_height), view.world_position);

        info.coordinate       = compute_morph(approximate_coordinate, approximate_view_distance);
        info.world_coordinate = compute_world_coordinate_precise(info.coordinate, approximate_world_coordinate.normal);
    } else {
#endif
        info.coordinate       = compute_morph(approximate_coordinate, approximate_view_distance);
        info.world_coordinate = compute_world_coordinate(info.coordinate);
#ifdef HIGH_PRECISION
    }
#endif

    info.blend = compute_blend(approximate_view_distance);

    return info;
}

fn vertex_output(info: ptr<function, VertexInfo>, height: f32) -> VertexOutput {
    let world_position = apply_height((*info).world_coordinate, height);

    var output: VertexOutput;
    output.clip_position  = position_world_to_clip(world_position);
    output.tile_index     = (*info).tile_index;
    output.coordinate_uv  = (*info).coordinate.uv;
    output.world_position = vec4<f32>(world_position, 1.0);
    output.world_normal   = (*info).world_coordinate.normal;
    return output;
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var info = vertex_info(input);

    let tile   = lookup_tile(info.coordinate, info.blend, 0u);
    var height = sample_height(tile);

    if (info.blend.ratio > 0.0) {
        let tile2 = lookup_tile(info.coordinate, info.blend, 1u);
        height    = mix(height, sample_height(tile2), info.blend.ratio);
    }

//    if (distance(info.world_position, view.world_position) > 3000000.0) {
//        height = 9000.0;
//    }
//    else {
//        height = -12000.0;
//    }

    // height = height * 30.0;

    return vertex_output(&info, height);
}
