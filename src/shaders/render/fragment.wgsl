#define_import_path bevy_terrain::fragment

#import bevy_terrain::types::{Blend, NodeLookup, Coordinate}
#import bevy_terrain::bindings::{config, view_config, tiles}
#import bevy_terrain::functions::{compute_coordinate, compute_blend, lookup_node}
#import bevy_terrain::attachments::{sample_normal, sample_color}
#import bevy_terrain::debug::{show_lod, show_tiles, show_quadtree, show_pixels}
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
    let view_distance = distance(input.world_position.xyz, view.world_position);

    var info: FragmentInfo;
    info.coordinate     = compute_coordinate(tiles[input.tile_index], input.coordinate_uv);
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

fn fragment_debug(info: ptr<function, FragmentInfo>, output: ptr<function, FragmentOutput>, lookup: NodeLookup, normal: vec3<f32>) {
#ifdef SHOW_LOD
    (*output).color = show_lod((*info).blend, lookup);
#endif
#ifdef SHOW_TILES
    (*output).color = show_tiles((*info).coordinate);
#endif
#ifdef SHOW_UV
    (*output).color = vec4<f32>(lookup.coordinate.uv, 0.0, 1.0);
#endif
#ifdef SHOW_QUADTREE
    (*output).color = show_quadtree((*info).coordinate);
#endif
#ifdef SHOW_PIXELS
    (*output).color = mix((*output).color, show_pixels(lookup), 0.5);
#endif
#ifdef SHOW_NORMALS
    (*output).color = vec4<f32>(normal, 1.0);
#endif

    // if ((*info).view_distance < view_config.precision_threshold_distance) {
    //     (*output).color = mix((*output).color, vec4<f32>(0.0, 1.0, 0.0, 1.0), 0.3);
    // }
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    var info = fragment_info(input);

    let lookup = lookup_node(info.coordinate, info.blend, 0u);
    var color  = sample_color(lookup);
    var normal = sample_normal(lookup, info.world_normal, info.coordinate.side);

    if (info.blend.ratio > 0.0) {
        let lookup2 = lookup_node(info.coordinate, info.blend, 1u);
        color       = mix(color,  sample_color(lookup2),                                           info.blend.ratio);
        normal      = mix(normal, sample_normal(lookup2, info.world_normal, info.coordinate.side), info.blend.ratio);
    }

    var output: FragmentOutput;
    fragment_output(&info, &output, color, normal);
    fragment_debug(&info, &output, lookup, normal);
    return output;
}

