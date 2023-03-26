#import bevy_terrain::types

struct TerrainConfig {
    lod_count: u32,
    height: f32,
    leaf_node_size: u32,
    terrain_size: u32,

    height_size: f32,
    minmax_size: f32,
    _empty: u32,
    _empty: u32,
    height_scale: f32,
    minmax_scale: f32,
    _empty: u32,
    _empty: u32,
    height_offset: f32,
    minmax_offset: f32,
    _empty: u32,
    _empty: u32,
}

// view bindings
#import bevy_pbr::mesh_view_bindings

// terrain view bindings
@group(1) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(1) @binding(1)
var quadtree: texture_2d_array<u32>;
@group(1) @binding(2)
var<storage> tiles: TileList;

// terrain bindings
@group(2) @binding(0)
var<uniform> config: TerrainConfig;
@group(2) @binding(1)
var atlas_sampler: sampler;
@group(2) @binding(2)
var height_atlas: texture_2d_array<f32>;
@group(2) @binding(3)
var minmax_atlas: texture_2d_array<f32>;

#import bevy_pbr::mesh_types
#import bevy_pbr::pbr_types

#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::pbr_ambient
#import bevy_pbr::shadows
#import bevy_pbr::fog
#import bevy_pbr::pbr_functions

#import bevy_terrain::node
#import bevy_terrain::functions
#import bevy_terrain::debug

struct FragmentData {
    world_normal: vec3<f32>,
    debug_color: vec4<f32>,
}

fn vertex_height(lookup: NodeLookup) -> f32 {
    let height_coords = lookup.atlas_coords * config.height_scale + config.height_offset;
    let height = textureSampleLevel(height_atlas, atlas_sampler, height_coords, lookup.atlas_index, 0.0).x;

    return height * config.height;
}

fn lookup_fragment_data(input: FragmentInput, lookup: NodeLookup, ddx: vec2<f32>, ddy: vec2<f32>) -> FragmentData {
    let atlas_lod = lookup.atlas_lod;
    let atlas_index = lookup.atlas_index;
    let atlas_coords = lookup.atlas_coords;
    let ddx = ddx / f32(1u << atlas_lod);
    let ddy = ddy / f32(1u << atlas_lod);

    let height_coords = atlas_coords * config.height_scale + config.height_offset;
    let height_ddx = ddx / 512.0;
    let height_ddy = ddy / 512.0;

    let world_normal = calculate_normal(height_coords, atlas_index, atlas_lod, height_ddx, height_ddy);

    var debug_color = vec4<f32>(0.5);

#ifdef SHOW_LOD
    debug_color = mix(debug_color, show_lod(atlas_lod, input.world_position.xyz), 0.4);
#endif

#ifdef SHOW_UV
    debug_color = mix(debug_color, vec4<f32>(atlas_coords.x, atlas_coords.y, 0.0, 1.0), 0.5);
#endif

    return FragmentData(world_normal, debug_color);
}

fn blend_fragment_data(data1: FragmentData, data2: FragmentData, blend_ratio: f32) -> FragmentData {
    let world_normal = mix(data2.world_normal, data1.world_normal, blend_ratio);
    let debug_color = mix(data2.debug_color, data1.debug_color, blend_ratio);

    return FragmentData(world_normal, debug_color);
}

fn process_fragment(input: FragmentInput, data: FragmentData) -> Fragment {
    let do_discard = input.local_position.x < 2.0 || input.local_position.x > f32(config.terrain_size) - 2.0 ||
                     input.local_position.y < 2.0 || input.local_position.y > f32(config.terrain_size) - 2.0;

    var color = mix(data.debug_color, vec4<f32>(input.debug_color.xyz, 1.0), input.debug_color.w);

#ifdef LIGHTING
    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance = 0.0;
    pbr_input.frag_coord = input.frag_coord;
    pbr_input.world_position = input.world_position;
    pbr_input.world_normal = data.world_normal;
    pbr_input.is_orthographic = view.projection[3].w == 1.0;
    pbr_input.N = data.world_normal;
    pbr_input.V = calculate_view(input.world_position, pbr_input.is_orthographic);
    color = pbr(pbr_input);
#endif

    return Fragment(color, do_discard);
}

#ifndef MINMAX
#import bevy_terrain::vertex
#else
#import bevy_terrain::minmax
#endif

#import bevy_terrain::fragment
