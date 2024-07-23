#define_import_path bevy_terrain::vertex

#import bevy_terrain::types::{Blend, AtlasTile, Coordinate}
#import bevy_terrain::bindings::{terrain, terrain_view, geometry_tiles}
#import bevy_terrain::functions::{lookup_tile, compute_tile_uv, compute_local_position, compute_relative_position, compute_morph, compute_blend, normal_local_to_world, position_local_to_world}
#import bevy_terrain::attachments::{sample_height}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::view_transformations::position_world_to_clip

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       tile_index: u32,
    @location(1)       coordinate_uv: vec2<f32>,
    @location(2)       world_position: vec4<f32>,
    @location(3)       world_normal: vec3<f32>,
}

struct VertexInfo {
    tile_index: u32,
    coordinate: Coordinate,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    blend: Blend,
}

fn vertex_info(input: VertexInput) -> VertexInfo {
    let tile_index                 = input.vertex_index / terrain_view.vertices_per_tile;
    let tile                       = geometry_tiles[tile_index];
    let tile_uv                    = compute_tile_uv(input.vertex_index);
    let approximate_coordinate     = Coordinate(tile.face, tile.lod, tile.xy, tile_uv);
    let approximate_local_position = compute_local_position(approximate_coordinate);
    let approximate_world_position = position_local_to_world(approximate_local_position);
    let approximate_world_normal   = normal_local_to_world(approximate_local_position);
    var approximate_view_distance  = distance(approximate_world_position + terrain_view.approximate_height * approximate_world_normal, view.world_position);

    var coordinate: Coordinate; var world_position: vec3<f32>; var world_normal: vec3<f32>;

#ifdef HIGH_PRECISION
    if (approximate_view_distance < terrain_view.precision_threshold_distance) {
        let approximate_relative_position = compute_relative_position(approximate_coordinate);
        approximate_view_distance         = length(approximate_relative_position + terrain_view.approximate_height * approximate_world_normal);

        coordinate            = compute_morph(approximate_coordinate, approximate_view_distance);
        let relative_position = compute_relative_position(coordinate);
        world_position        = view.world_position + relative_position;
        world_normal          = approximate_world_normal;
    } else {
#endif
        coordinate         = compute_morph(approximate_coordinate, approximate_view_distance);
        let local_position = compute_local_position(coordinate);
        world_position     = position_local_to_world(local_position);
        world_normal       = normal_local_to_world(local_position);
#ifdef HIGH_PRECISION
    }
#endif

    var info: VertexInfo;
    info.tile_index     = tile_index;
    info.coordinate     = coordinate;
    info.world_position = world_position;
    info.world_normal   = world_normal;
    info.blend          = compute_blend(approximate_view_distance);

    return info;
}

fn vertex_output(info: ptr<function, VertexInfo>, height: f32) -> VertexOutput {
    let world_position = (*info).world_position + height * (*info).world_normal;

    var output: VertexOutput;
    output.clip_position  = position_world_to_clip(world_position);
    output.tile_index     = (*info).tile_index;
    output.coordinate_uv  = (*info).coordinate.uv;
    output.world_position = vec4<f32>(world_position, 1.0);
    output.world_normal   = (*info).world_normal;
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

    return vertex_output(&info, height);
}
