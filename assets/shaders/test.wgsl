#import bevy_terrain::types::{NodeLookup, Blend, UVCoordinate, LookupInfo}
#import bevy_terrain::bindings::{config, view_config, atlas_sampler, attachments, attachment0_atlas, attachment1_atlas}
#import bevy_terrain::functions::{lookup_attachment_group, node_count, grid_offset, tile_coordinate, compute_morph, local_position_from_coordinate, compute_blend, quadtree_lod, lookup_node, local_to_world_position, world_to_clip_position}
#import bevy_terrain::attachments::{sample_attachment0, sample_attachment1, sample_height_grad, sample_normal_grad, sample_attachment1_gather0}
#import bevy_terrain::vertex::{VertexInput, vertex_lookup_info}
#import bevy_terrain::fragment::{FragmentOutput}
#import bevy_terrain::debug::{show_lod, show_tiles, show_pixels}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

fn vertex_output(input: VertexInput, info: LookupInfo, height: f32) -> VertexOutput {
    var output: VertexOutput;

    let local_position = local_position_from_coordinate(info.coordinate, height);
    let view_distance  = distance(local_position, view_config.view_local_position);

    output.side              = info.coordinate.side;
    output.uv                = info.coordinate.uv;
    output.view_distance     = view_distance;
    output.world_normal      = normalize(local_position);
    output.world_position    = local_to_world_position(local_position);
    output.fragment_position = world_to_clip_position(output.world_position);


#ifdef SHOW_TILES
    output.debug_color       = show_tiles(info.view_distance, input.vertex_index);
#endif

    return output;
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

struct VertexOutput {
    @builtin(position)       fragment_position: vec4<f32>,
    @location(0)             side: u32,
    @location(1)             uv: vec2<f32>,
    @location(2)             view_distance: f32,
    @location(3)             world_normal: vec3<f32>,
    @location(4)             world_position: vec4<f32>,
    @location(5)             debug_color: vec4<f32>,
    @location(6)             local_available: f32,
}

struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       fragment_position: vec4<f32>,
    @location(0)             side: u32,
    @location(1)             uv: vec2<f32>,
    @location(2)             view_distance: f32,
    @location(3)             world_normal: vec3<f32>,
    @location(4)             world_position: vec4<f32>,
    @location(5)             debug_color: vec4<f32>,
    @location(6)             local_available: f32,
}

@group(3) @binding(0)
var gradient1: texture_1d<f32>;
@group(3) @binding(1)
var gradient1_sampler: sampler;
@group(3) @binding(2)
var gradient2: texture_1d<f32>;
@group(3) @binding(3)
var gradient2_sampler: sampler;

fn local_available(lookup: NodeLookup) -> bool {
    let attachment = attachments[1];
    let coordinate = lookup.coordinate * attachment.scale + attachment.offset;
    let gather = textureGather(0, attachment1_atlas, atlas_sampler, coordinate, lookup.index);
    return all(gather != vec4<f32>(0.0));
}

fn sample_height(lookup_global: NodeLookup, lookup_local: NodeLookup, local: bool, offset: vec2<f32>) -> f32 {
    var height: f32;

    if (local) {
        let attachment = attachments[1];
        let coordinate = lookup_local.coordinate * attachment.scale + attachment.offset;
        height = textureSampleLevel(attachment1_atlas, atlas_sampler, coordinate + offset, lookup_local.index, 0.0).x;
    }
    else {
        let attachment = attachments[1];
        let coordinate = lookup_local.coordinate * attachment.scale + attachment.offset;
        let gather = textureGather(0, attachment1_atlas, atlas_sampler, coordinate, lookup_local.index);
        height = max(max(gather.x, gather.y), max(gather.z, gather.w));
    }

    return mix(config.min_height, config.max_height, height);
}

fn sample_color(lookup_global: NodeLookup, lookup_local: NodeLookup, local: bool) -> vec4<f32> {
    let height = sample_height(lookup_global, lookup_local, local, vec2(0.0));

    var color: vec4<f32>;

    if (local) {
        color = textureSampleLevel(gradient2, gradient2_sampler, mix(0.0, 1.0, height / config.min_height), 0.0);
    } else {
        if (height < 0.0) {
            color = textureSampleLevel(gradient1, gradient1_sampler, mix(0.0, 0.075, pow(height / config.min_height, 0.25)), 0.0);
        } else {
            color = textureSampleLevel(gradient1, gradient1_sampler, mix(0.09, 1.0, pow(height / config.max_height * 2.0, 1.0)), 0.0);
        }
    }

    return color;
}

fn sample_normal(lookup_global: NodeLookup, lookup_local: NodeLookup, local: bool, vertex_normal: vec3<f32>, side: u32) -> vec3<f32> {
    let height_attachment = attachments[0];

    var pixels_per_side: f32;

    if (local) { pixels_per_side = height_attachment.size * node_count(lookup_local.lod); }
    else       { pixels_per_side = height_attachment.size * node_count(lookup_global.lod); }


#ifdef SPHERICAL
    var FACE_UP = array(
        vec3( 0.0, 1.0,  0.0),
        vec3( 0.0, 1.0,  0.0),
        vec3( 0.0, 0.0, -1.0),
        vec3( 0.0, 0.0, -1.0),
        vec3(-1.0, 0.0,  0.0),
        vec3(-1.0, 0.0,  0.0),
    );

    let face_up = FACE_UP[side];

    let normal    = normalize(vertex_normal);
    let tangent   = cross(face_up, normal);
    let bitangent = cross(normal, tangent);
    let TBN       = mat3x3(tangent, bitangent, normal);

    let side_length = 3.14159265359 / 4.0;
#else
    let TBN = mat3x3(1.0, 0.0, 0.0,
                     0.0, 0.0, 1.0,
                     0.0, 1.0, 0.0);

    let side_length = 1.0;
#endif

    // Todo: this is only an approximation of the S2 distance (pixels are not spaced evenly and they are not perpendicular)
    let distance_between_samples = side_length / pixels_per_side;
    let offset = 0.5 / height_attachment.size;

    let left  = sample_height(lookup_global, lookup_local, local, vec2<f32>(-offset,     0.0));
    let up    = sample_height(lookup_global, lookup_local, local, vec2<f32>(    0.0, -offset));
    let right = sample_height(lookup_global, lookup_local, local, vec2<f32>( offset,     0.0));
    let down  = sample_height(lookup_global, lookup_local, local, vec2<f32>(    0.0,  offset));

    let surface_normal = normalize(vec3<f32>(left - right, down - up, distance_between_samples));

    return normalize(TBN * surface_normal);
}

fn sample_test(lookup: NodeLookup) -> f32 {
    let attachment = attachments[1];
    let coordinate = lookup.coordinate * attachment.scale + attachment.offset;
    let height = textureSampleLevel(attachment1_atlas, atlas_sampler, coordinate, lookup.index, 0.0).x;
    return mix(config.min_height, config.max_height, height);
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let grid_index = input.vertex_index % view_config.vertices_per_tile;
    let grid_offset1 = grid_offset(grid_index);
    let info   = vertex_lookup_info(input);
    let lookup = lookup_attachment_group(info, 0u, 1u);
    var valid  = local_available(lookup);
    var height = sample_test(lookup);

    var color = vec4(0.0);

    // if (info.blend_ratio > 0.0) {
    //     let lookup2 = lookup_attachment_group(info, 1u, 1u);
    //     valid       = valid && local_available(lookup2);
    //     height      = mix(height, sample_test(lookup2), info.blend_ratio);
    // }

    // if (!valid) {
    //    height = height / 0.0;
    // }

    height = -0.001;

    var output = vertex_output(input, info, height);
    output.local_available = select(0.0, 1.0, valid);
    output.debug_color = color;
    return output;
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    let info = fragment_lookup_info(input);
    var local = input.local_available > 0.999;

    let lookup_global = lookup_attachment_group(info, 0u, 0u);
    let lookup_local  = lookup_attachment_group(info, 0u, 1u);
    local = local_available(lookup_local);
    var normal        = sample_normal(lookup_global, lookup_local, local, input.world_normal, input.side);
    var color         = sample_color(lookup_global, lookup_local, local);

    if (info.blend_ratio > 0.0) {
        let lookup_global2 = lookup_attachment_group(info, 1u, 0u);
        let lookup_local2  = lookup_attachment_group(info, 1u, 1u);
        color              = mix(color,  sample_color(lookup_global2, lookup_local2, local),                 info.blend_ratio);
        normal             = mix(normal, sample_normal(lookup_global2, lookup_local2, local, input.world_normal, input.side), info.blend_ratio);
    }

    // if (input.local_available < 0.9999) {
    //     normal = vec3(0.0, 0.0, 0.0);
    //     color = vec4(0.3);
    //     // color  = sample_color(lookup_global, lookup_local, false);
    // }

    if (!local) {
        discard;
    }

    // if (input.local_available < 0.9999) { discard; }

    return fragment_output(input, color, normal, lookup_local);
}
