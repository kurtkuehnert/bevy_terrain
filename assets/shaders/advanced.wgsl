#import bevy_terrain::types

// Customize your attachment offsets and scales here.
// They are used to line up the uvs of adjacent nodes with a border size different from 0.
struct TerrainConfig {
    lod_count: u32,
    height: f32,
    leaf_node_size: u32,
    terrain_size: u32,

    height_size: f32,
    minmax_size: f32,
    albedo_size: f32,
    _empty: f32,
    height_scale: f32,
    minmax_scale: f32,
    albedo_scale: f32,
    _empty: f32,
    height_offset: f32,
    minmax_offset: f32,
    albedo_offset: f32,
    _empty: f32,
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
// Customize your attachments here.
@group(2) @binding(2)
var height_atlas: texture_2d_array<f32>;
@group(2) @binding(3)
var minmax_atlas: texture_2d_array<f32>;
#ifdef ALBEDO
@group(2) @binding(4)
var albedo_atlas: texture_2d_array<f32>;
#endif

// Customize your material data here.
@group(3) @binding(0)
var array_texture: texture_2d_array<f32>;
@group(3) @binding(1)
var array_sampler: sampler;

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

// The terrain data required by your `fragment_color` function.
// This data will be fetched from the atlases by means of the `AtlasLookup`.
// To smoothen the transition between different lods the fragment data will be blended at the fringe between them.
struct FragmentData {
    world_normal: vec3<f32>,
    color: vec4<f32>,
}

// Lookup the terrain data required by your `fragment_color` function.
// This will happen once or twice (lod fringe).
fn lookup_fragment_data(input: FragmentInput, lookup: NodeLookup, ddx: vec2<f32>, ddy: vec2<f32>) -> FragmentData {
    let atlas_lod = lookup.atlas_lod;
    let atlas_index = lookup.atlas_index;
    let atlas_coords = lookup.atlas_coords;
    let ddx = ddx / f32(1u << atlas_lod);
    let ddy = ddy / f32(1u << atlas_lod);

    // Adjust the uvs and deltas for your attachments.
    let height_coords = atlas_coords * config.height_scale + config.height_offset;
    let height_ddx = ddx / config.height_size;
    let height_ddy = ddy / config.height_size;
    let albedo_coords = atlas_coords * config.albedo_scale + config.albedo_offset;
    let albedo_ddx = ddx / config.albedo_size;
    let albedo_ddy = ddy / config.albedo_size;

    // Calculate the normal from the heightmap.
    let world_normal = calculate_normal(height_coords, atlas_index, atlas_lod, height_ddx, height_ddy);

#ifdef ALBEDO
#ifdef SAMPLE_GRAD
    var color = textureSampleGrad(albedo_atlas, atlas_sampler, albedo_coords, atlas_index, albedo_ddx, albedo_ddy);
#else
    var color = textureSample(albedo_atlas, atlas_sampler, albedo_coords, atlas_index);
    // var color = textureSampleLevel(albedo_atlas, atlas_sampler, albedo_coords, atlas_index, 0.0);
#endif

#else
    var color = vec4<f32>(0.5);
#endif

#ifdef SHOW_LOD
    color = mix(color, show_lod(atlas_lod, input.world_position.xyz), 0.4);
#endif

    return FragmentData(world_normal, color);
}

// Blend the terrain data at the fringe between two lods.
fn blend_fragment_data(data1: FragmentData, data2: FragmentData, blend_ratio: f32) -> FragmentData {
    let world_normal =  mix(data2.world_normal, data1.world_normal, blend_ratio);
    let color = mix(data2.color, data1.color, blend_ratio);

    return FragmentData(world_normal, color);
}

// The function that evaluates the color of the fragment.
// It will be called once in the fragment shader with the blended fragment data.
fn process_fragment(in: FragmentInput, data: FragmentData) -> Fragment {
    let world_normal = data.world_normal;
    var color = data.color;

#ifndef ALBEDO
    let height = in.world_position.y / config.height;
    let slope = world_normal.y;

    let min_slope = 0.6;
    let max_slope = 1.0;
    let slope_weight = (max(min(slope, max_slope), min_slope) - min_slope) / (max_slope - min_slope);

    let min_height = 0.7;
    let max_height = 0.9;
    let height_weight = (max(min(height, max_height), min_height) - min_height) / (max_height - min_height);

    // Sample your custom material.
    let uv = in.local_position / 10.0f;
    let grass = textureSample(array_texture, array_sampler, uv, 0);
    let rock = textureSample(array_texture, array_sampler, uv, 1);
    let snow = textureSample(array_texture, array_sampler, uv, 2);
    let sand = textureSample(array_texture, array_sampler, uv, 3);

    color = mix(mix(sand, rock, slope_weight), snow, height_weight);
#endif

#ifdef LIGHTING
    // Finally assemble the pbr input and calculate the lighting.
    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.material.perceptual_roughness = 0.6;
    pbr_input.material.reflectance = 0.1;
    pbr_input.frag_coord = in.frag_coord;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = world_normal;
    pbr_input.is_orthographic = view.projection[3].w == 1.0;
    pbr_input.N = world_normal;
    pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);

    color = pbr(pbr_input);
#endif

    return Fragment(color, false);
}

#import bevy_terrain::fragment
