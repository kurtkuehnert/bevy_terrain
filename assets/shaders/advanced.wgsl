#import bevy_terrain::types TerrainViewConfig
#import bevy_terrain::types TileList

 #import bevy_pbr::pbr_functions as pbr_functions
 

#import bevy_pbr::pbr_types as pbr_types

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
#import bevy_pbr::ambient
#import bevy_pbr::shadows
#import bevy_pbr::fog
#import bevy_pbr::pbr_functions
 
#import bevy_terrain::functions FragmentInput, blend_fragment_data
#import bevy_terrain::debug

#import bevy_terrain::fragment fragment
#import bevy_terrain::vertex vertex

// The terrain data required by your `fragment_color` function.
// This data will be fetched from the atlases by means of the `AtlasLookup`.
// To smoothen the transition between different lods the fragment data will be blended at the fringe between them.
struct FragmentData {
    world_normal: vec3<f32>,
    color: vec4<f32>,
}
 
 

