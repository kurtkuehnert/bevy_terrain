#define_import_path bevy_terrain::fragment

#import bevy_terrain::types::{LookupInfo, NodeLookup}
#import bevy_terrain::functions::{compute_blend, lookup_node, s2_from_local_position}
#import bevy_terrain::attachments::{sample_normal_grad, sample_color_grad}
#import bevy_terrain::debug::{show_lod, show_quadtree, show_pixels}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       fragment_position: vec4<f32>,
    @location(0)             local_position: vec3<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}

struct FragmentOutput {
    @location(0)             color: vec4<f32>
}

fn lookup_info_fragment(local_position: vec3<f32>) -> LookupInfo {
    let s2     = s2_from_local_position(local_position);
    let blend  = compute_blend(local_position);
    var ddx    = dpdx(s2.st);
    var ddy    = dpdy(s2.st);
    let ddside = max(abs(dpdx(f32(s2.side))), abs(dpdy(f32(s2.side))));

    if (ddside > 0.0) {
        ddx = vec2<f32>(0.0);
        ddy = vec2<f32>(0.0);
    }

    return LookupInfo(s2, blend.lod, blend.ratio, ddx, ddy);
}

fn fragment_output(input: FragmentInput, color: vec4<f32>, normal: vec3<f32>, lookup: NodeLookup) -> FragmentOutput {
    var output: FragmentOutput;

    output.color = color;

#ifdef SHOW_LOD
    output.color = show_lod(input.local_position, lookup.atlas_lod);
#endif
#ifdef SHOW_UV
    output.color = vec4<f32>(lookup.atlas_coordinate, 0.0, 1.0);
#endif
#ifdef SHOW_TILES
    output.color = input.debug_color;
#endif
#ifdef SHOW_QUADTREE
    output.color = show_quadtree(input.local_position);
#endif
#ifdef SHOW_PIXELS
    output.color = mix(output.color, show_pixels(input.local_position, lookup.atlas_lod), 0.5);
#endif
#ifdef SHOW_NORMALS
    output.color = vec4<f32>(normal, 1.0);
#endif

    return output;
}

@fragment
fn default_fragment(input: FragmentInput) -> FragmentOutput {
    let info = lookup_info_fragment(input.local_position);

    let lookup = lookup_node(info, 0u);
    var normal = sample_normal_grad(lookup, input.local_position);
    var color  = sample_color_grad(lookup);

    if (info.blend_ratio > 0.0) {
        let lookup2 = lookup_node(info, 1u);
        normal      = mix(normal, sample_normal_grad(lookup2, input.local_position), info.blend_ratio);
        color       = mix(color,  sample_color_grad(lookup2),                        info.blend_ratio);
    }

#ifdef LIGHTING
    var pbr_input: PbrInput                 = pbr_input_new();
    pbr_input.material.base_color           = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance          = 0.0;
    pbr_input.frag_coord                    = input.fragment_position;
    pbr_input.world_position                = input.world_position;
    pbr_input.world_normal                  = normal;
    pbr_input.N                             = normal;
    pbr_input.V                             = calculate_view(input.world_position, pbr_input.is_orthographic);
    color = apply_pbr_lighting(pbr_input);
#endif

    return fragment_output(input, color, normal, lookup);
}
