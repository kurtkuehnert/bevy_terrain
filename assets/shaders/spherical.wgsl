#import bevy_terrain::types::NodeLookup
#import bevy_terrain::bindings::config
#import bevy_terrain::functions::{vertex_coordinate, lookup_node}
#import bevy_terrain::attachments::{sample_height_grad, sample_normal_grad}
#import bevy_terrain::vertex::{VertexInput, VertexOutput, VertexInfo, setup_vertex_info, high_precision, apply_height}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, FragmentInfo}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

#import bevy_terrain::bindings::{view_config, tiles, model_view_approximation, quadtree, atlas_sampler, attachments, attachment0_atlas}
#import bevy_terrain::types::{Blend, Coordinate, LookupInfo, Tile, Morph}
#import bevy_terrain::functions::{tile_size, compute_coordinate, compute_local_position, compute_relative_coordinate, compute_relative_position, compute_grid_offset, compute_morph, compute_blend, local_to_world_position, world_to_clip_position}
#import bevy_terrain::attachments::{sample_height}
#import bevy_terrain::debug::{show_lod, show_tiles, show_quadtree, index_color}
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

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var info: VertexInfo;

    setup_vertex_info(&info, input);
    high_precision(&info);

    let blend = compute_blend(info.view_distance);

    apply_height(&info, blend); // terrain data sample hook

    var output: VertexOutput;
    output.tile_index     = info.tile_index;
    output.offset    = info.offset;
    output.view_distance  = info.view_distance;
    output.world_normal   = info.world_normal;
    output.world_position = vec4<f32>(info.world_position, 1.0);
    output.clip_position  = world_to_clip_position(info.world_position);

#ifdef SHOW_TILES
    output.debug_color    = show_tiles(&info);
#endif

    return output;
}

fn setup_fragment_info(info: ptr<function, FragmentInfo>, input: FragmentInput) {
    (*info).tile   = tiles[input.tile_index];
    (*info).offset = input.offset;
    (*info).blend  = compute_blend(input.view_distance);
    (*info).clip_position  = input.clip_position;
    (*info).world_normal   = input.world_normal;
    (*info).world_position = input.world_position;
    (*info).debug_color    = input.debug_color;
}

fn fragment_pbr(info: ptr<function, FragmentInfo>, output: ptr<function, FragmentOutput>, color: vec4<f32>, normal: vec3<f32>) {
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
    (*output).color = show_quadtree(coordinate);
#endif
#ifdef SHOW_PIXELS
    (*output).color = mix((*output).color, show_pixels(coordinate, lookup.lod), 0.5);
#endif
#ifdef SHOW_NORMALS
    (*output).color = vec4<f32>(normal, 1.0);
#endif
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    var info: FragmentInfo;
    setup_fragment_info(&info, input);

    var color: vec4<f32>;
    var normal: vec3<f32>;

    let lookup = lookup_node(info.tile, info.offset, info.blend, 0u);
    color = sample_color_grad(lookup);
    normal = sample_normal_grad(lookup, info.world_normal, info.tile.side);

    if (info.blend.ratio > 0.0) {
        let lookup2 = lookup_node(info.tile, info.offset, info.blend, 1u);
        color       = mix(color,  sample_color_grad(lookup2),                                     info.blend.ratio);
        normal      = mix(normal, sample_normal_grad(lookup2, info.world_normal, info.tile.side), info.blend.ratio);
    }

    var output: FragmentOutput;
    fragment_pbr(&info, &output, color, normal);
    fragment_debug(&info, &output, lookup, normal);
    return output;
}
