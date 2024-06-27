#define_import_path bevy_terrain::fragment

#import bevy_terrain::types::{Tile, NodeLookup, Blend}
#import bevy_terrain::bindings::{tiles, config, view_config}
#import bevy_terrain::functions::{compute_blend, lookup_node}
#import bevy_terrain::debug::{show_lod, show_quadtree, show_pixels}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       clip_position: vec4<f32>,
    @location(0)             tile_index: u32,
    @location(1)             offset: vec2<f32>,
    @location(2)             view_distance: f32,
    @location(3)             world_normal: vec3<f32>,
    @location(4)             world_position: vec4<f32>,
    @location(5)             debug_color: vec4<f32>,
}

struct FragmentOutput {
    @location(0)             color: vec4<f32>
}

struct FragmentInfo {
    tile: Tile,
    offset: vec2<f32>,
    offset_dx: vec2<f32>,
    offset_dy: vec2<f32>,
    view_distance: f32,
    blend: Blend,
    clip_position: vec4<f32>,
    world_normal: vec3<f32>,
    world_position: vec4<f32>,
    debug_color: vec4<f32>,
    color: vec4<f32>,
    normal: vec3<f32>,
}

fn fragment_info(input: FragmentInput) -> FragmentInfo{
    var info: FragmentInfo;
    info.tile           = tiles[input.tile_index];
    info.offset         = input.offset;
    info.offset_dx      = dpdx(input.offset);
    info.offset_dy      = dpdy(input.offset);
    info.view_distance  = input.view_distance;
    info.blend          = compute_blend(input.view_distance);
    info.clip_position  = input.clip_position;
    info.world_normal   = input.world_normal;
    info.world_position = input.world_position;
    info.debug_color    = input.debug_color;

    return info;
}

fn fragment_lookup_node(info: ptr<function, FragmentInfo>, lod_offset: u32) -> NodeLookup {
    return lookup_node((*info).tile, (*info).offset, (*info).offset_dx, (*info).offset_dy, (*info).blend, lod_offset);
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
#ifdef SHOW_UV
    (*output).color = vec4<f32>(lookup.uv, 0.0, 1.0);
#endif
#ifdef SHOW_TILES
    (*output).color = (*info).debug_color;
#endif
#ifdef SHOW_QUADTREE
    (*output).color = show_quadtree((*info).tile, (*info).offset);
#endif
#ifdef SHOW_PIXELS
    (*output).color = mix((*output).color, show_pixels(lookup), 0.5);
#endif
#ifdef SHOW_NORMALS
    (*output).color = vec4<f32>(normal, 1.0);
#endif

    if ((*info).view_distance < view_config.precision_threshold_distance * config.scale) {
        (*output).color = mix((*output).color, vec4<f32>(1.0, 0.0, 0.0, 1.0), 0.3);
    }

    // (*output).color = vec4<f32>((*info).offset_dx * 4.0, 0.0, 1.0);
    // (*output).color = vec4<f32>(lookup.ddx * 50.0, 0.0, 1.0);
}