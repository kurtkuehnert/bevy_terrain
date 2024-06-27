#import bevy_terrain::bindings::config
#import bevy_terrain::attachments::{sample_height_grad, sample_normal_grad}
#import bevy_terrain::vertex::{VertexInput, VertexOutput, VertexInfo, vertex_info, vertex_lookup_node, vertex_output, vertex_debug}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, FragmentInfo, fragment_info, fragment_lookup_node, fragment_output, fragment_debug}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

#import bevy_terrain::bindings::{view_config, tiles, terrain_model_approximation, quadtree, atlas_sampler, attachments, attachment0_atlas}
#import bevy_terrain::types::{Blend, Coordinate, NodeLookup, Tile, Morph}
#import bevy_terrain::functions::{tile_size, compute_coordinate, compute_local_position, compute_relative_coordinate, compute_relative_position, compute_grid_offset, compute_morph, compute_blend, position_local_to_world}
#import bevy_terrain::attachments::{sample_height}
#import bevy_terrain::debug::{show_lod, show_tiles, show_quadtree, index_color}
#import bevy_pbr::mesh_view_bindings::view

@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient_sampler: sampler;

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
    var info = vertex_info(input);

    let lookup = vertex_lookup_node(&info, 0u);
    var height = sample_height(lookup);

    if (info.blend.ratio > 0.0) {
        let lookup2 = vertex_lookup_node(&info, 1u);
        height      = mix(height, sample_height(lookup2), info.blend.ratio);
    }

    var output: VertexOutput;
    vertex_output(&info, &output, height);
    vertex_debug(&info, &output);
    return output;
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    var info = fragment_info(input);

    let lookup = fragment_lookup_node(&info, 0u);
    var color  = sample_color_grad(lookup);
    var normal = sample_normal_grad(lookup, info.world_normal, info.tile.side);

    if (info.blend.ratio > 0.0) {
        let lookup2 = fragment_lookup_node(&info, 1u);
        color       = mix(color,  sample_color_grad(lookup2),                                     info.blend.ratio);
        normal      = mix(normal, sample_normal_grad(lookup2, info.world_normal, info.tile.side), info.blend.ratio);
    }

    var output: FragmentOutput;
    fragment_output(&info, &output, color, normal);
    fragment_debug(&info, &output, lookup, normal);

    return output;
}
