#define_import_path bevy_terrain::fragment

#import bevy_terrain::types::{LookupInfo, NodeLookup, UVCoordinate, Blend}
#import bevy_terrain::functions::{compute_blend, lookup_node, quadtree_lod}
#import bevy_terrain::attachments::{sample_normal_grad, sample_color_grad}
#import bevy_terrain::debug::{show_lod, show_quadtree, show_pixels}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       fragment_position: vec4<f32>,
    @location(0)             side: u32,
    @location(1)             uv: vec2<f32>,
    @location(2)             view_distance: f32,
    @location(3)             world_normal: vec3<f32>,
    @location(4)             world_position: vec4<f32>,
    @location(5)             debug_color: vec4<f32>,
}

struct FragmentOutput {
    @location(0)             color: vec4<f32>
}

fn fragment_lookup_info(input: FragmentInput) -> LookupInfo {
    let coordinate    = UVCoordinate(input.side, input.uv);
    let ddx           = dpdx(input.uv);
    let ddy           = dpdy(input.uv);
    let view_distance = input.view_distance;

#ifdef QUADTREE_LOD
    let blend = Blend(quadtree_lod(coordinate), 0.0);
#else
    let blend = compute_blend(view_distance);
#endif

    return LookupInfo(coordinate, view_distance, blend.lod, blend.ratio, ddx, ddy);
}

fn fragment_output(input: FragmentInput, color: vec4<f32>, normal: vec3<f32>, lookup: NodeLookup) -> FragmentOutput {
    var output: FragmentOutput;

    let coordinate = UVCoordinate(input.side, input.uv);

    output.color = color;

#ifdef LIGHTING
    var pbr_input: PbrInput                 = pbr_input_new();
    pbr_input.material.base_color           = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance          = 0.0;
    pbr_input.frag_coord                    = input.fragment_position;
    pbr_input.world_position                = input.world_position;
    pbr_input.world_normal                  = input.world_normal;
    pbr_input.N                             = normal;
    pbr_input.V                             = calculate_view(input.world_position, pbr_input.is_orthographic);

    output.color = apply_pbr_lighting(pbr_input);
#endif

#ifdef SHOW_LOD
    output.color = show_lod(coordinate, input.view_distance, lookup.lod);
#endif
#ifdef SHOW_UV
    output.color = vec4<f32>(lookup.coordinate, 0.0, 1.0);
#endif
#ifdef SHOW_TILES
    output.color = input.debug_color;
#endif
#ifdef SHOW_QUADTREE
    output.color = show_quadtree(coordinate);
#endif
#ifdef SHOW_PIXELS
    output.color = mix(output.color, show_pixels(coordinate, lookup.lod), 0.5);
#endif
#ifdef SHOW_NORMALS
    output.color = vec4<f32>(normal, 1.0);
#endif

    return output;
}

@fragment
fn default_fragment(input: FragmentInput) -> FragmentOutput {
    let info = fragment_lookup_info(input);

    let lookup = lookup_node(info, 0u);
    var normal = sample_normal_grad(lookup, input.world_normal, input.side);
    var color  = sample_color_grad(lookup);

    if (info.blend_ratio > 0.0) {
        let lookup2 = lookup_node(info, 1u);
        normal      = mix(normal, sample_normal_grad(lookup2, input.world_normal, input.side), info.blend_ratio);
        color       = mix(color,  sample_color_grad(lookup2),                                  info.blend_ratio);
    }

    return fragment_output(input, color, normal, lookup);
}
