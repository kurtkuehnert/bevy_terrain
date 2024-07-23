#define_import_path bevy_terrain::fragment

#import bevy_terrain::types::{Blend, AtlasTile, Coordinate}
#import bevy_terrain::bindings::{terrain, terrain_view, geometry_tiles}
#import bevy_terrain::functions::{compute_blend, lookup_tile}
#import bevy_terrain::attachments::{sample_normal, sample_color}
#import bevy_terrain::debug::{show_data_lod, show_geometry_lod, show_tile_tree, show_pixels}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

struct FragmentInput {
    @builtin(position)     clip_position: vec4<f32>,
    @location(0)           tile_index: u32,
    @location(1)           coordinate_uv: vec2<f32>,
    @location(2)           world_position: vec4<f32>,
    @location(3)           world_normal: vec3<f32>,
}

struct FragmentOutput {
    @location(0)             color: vec4<f32>
}

struct FragmentInfo {
    coordinate: Coordinate,
    view_distance: f32,
    blend: Blend,
    clip_position: vec4<f32>,
    world_normal: vec3<f32>,
    world_position: vec4<f32>,
    color: vec4<f32>,
    normal: vec3<f32>,
}

fn fragment_info(input: FragmentInput) -> FragmentInfo{
    let tile          = geometry_tiles[input.tile_index];
    let uv            = input.coordinate_uv;
    let view_distance = distance(input.world_position.xyz, view.world_position);

    var info: FragmentInfo;
    info.coordinate     = Coordinate(tile.face, tile.lod, tile.xy, uv, dpdx(uv), dpdy(uv));
    info.view_distance  = view_distance;
    info.blend          = compute_blend(view_distance);
    info.clip_position  = input.clip_position;
    info.world_normal   = input.world_normal;
    info.world_position = input.world_position;

    return info;
}

fn fragment_output(info: ptr<function, FragmentInfo>, output: ptr<function, FragmentOutput>, color: vec4<f32>, normal: vec3<f32>) {
#ifdef LIGHTING
    var pbr_input: PbrInput                 = pbr_input_new();
    pbr_input.material.base_color           = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance          = 0.0;
    pbr_input.frag_coord                    = (*info).clip_position;
    pbr_input.world_position                = (*info).world_position;
    pbr_input.world_normal                  = (*info).world_normal;
    pbr_input.N                             = normal;
    pbr_input.V                             = calculate_view((*info).world_position, pbr_input.is_orthographic);

    (*output).color = apply_pbr_lighting(pbr_input);
#else
    (*output).color = color;
#endif
}

fn fragment_debug(info: ptr<function, FragmentInfo>, output: ptr<function, FragmentOutput>, tile: AtlasTile, normal: vec3<f32>) {
#ifdef SHOW_DATA_LOD
    (*output).color = show_data_lod((*info).blend, tile);
#endif
#ifdef SHOW_GEOMETRY_LOD
    (*output).color = show_geometry_lod((*info).coordinate);
#endif
#ifdef SHOW_TILE_TREE
    (*output).color = show_tile_tree((*info).coordinate);
#endif
#ifdef SHOW_PIXELS
    (*output).color = mix((*output).color, show_pixels(tile), 0.5);
#endif
#ifdef SHOW_UV
    (*output).color = vec4<f32>(tile.coordinate.uv, 0.0, 1.0);
#endif
#ifdef SHOW_NORMALS
    (*output).color = vec4<f32>(normal, 1.0);
#endif

    // Todo: move this somewhere else
    if ((*info).view_distance < terrain_view.precision_threshold_distance) {
        (*output).color = mix((*output).color, vec4<f32>(0.1), 0.7);
    }
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    var info = fragment_info(input);

    let tile   = lookup_tile(info.coordinate, info.blend, 0u);
    var color  = sample_color(tile);
    var normal = sample_normal(tile, info.world_normal);

    if (info.blend.ratio > 0.0) {
        let tile2 = lookup_tile(info.coordinate, info.blend, 1u);
        color     = mix(color,  sample_color(tile2),                     info.blend.ratio);
        normal    = mix(normal, sample_normal(tile2, info.world_normal), info.blend.ratio);
    }

    var output: FragmentOutput;
    fragment_output(&info, &output, color, normal);
    fragment_debug(&info, &output, tile, normal);
    return output;
}

