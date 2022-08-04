#import bevy_terrain::types

// Customize your attachment offsets and scales here.
// They are used to line up the uvs of adjacent nodes with a border size different from 0.
struct TerrainConfig {
    lod_count: u32,
    height: f32,
    chunk_size: u32,
    terrain_size: u32,

    height_scale: f32,
    density_scale: f32,
    _empty: u32,
    _empty: u32,
    height_offset: f32,
    density_offset: f32,
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
var terrain_sampler: sampler;
// Customize your attachments here.
@group(2) @binding(2)
var height_atlas: texture_2d_array<f32>;
@group(2) @binding(3)
var density_atlas: texture_2d_array<f32>;

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
#import bevy_pbr::shadows
#import bevy_pbr::pbr_functions

#import bevy_terrain::atlas
#import bevy_terrain::functions
#import bevy_terrain::debug

// The terrain data required by your `fragment_color` function.
// This data will be fetched from the atlases by means of the `AtlasLookup`.
// To smoothen the transition between different lods the fragment data will be blended at the fringe between them.
struct FragmentData {
    world_normal: vec3<f32>,
}

// Lookup the terrain data required by your `fragment_color` function.
// This will happen once or twice (lod fringe).
fn lookup_fragment_data(in: FragmentInput, lookup: AtlasLookup) -> FragmentData {
    let lod = lookup.lod;
    let atlas_index = lookup.atlas_index;
    let atlas_coords = lookup.atlas_coords;

    // Adjust the uvs for your attachments.
    let height_coords = atlas_coords * config.height_scale + config.height_offset;

    // Calculate the normal from the heightmap.
    let world_normal = calculate_normal(height_coords, atlas_index, lod);

    return FragmentData(world_normal);
}

// Blend the terrain data at the fringe between two lods.
fn blend_fragment_data(data1: FragmentData, data2: FragmentData, blend_ratio: f32) -> FragmentData {
    let world_normal =  mix(data2.world_normal, data1.world_normal, blend_ratio);

    return FragmentData(world_normal);
}

// The function that evaluates the color of the fragment.
// It will be called once in the fragment shader with the blended fragment data.
fn fragment_color(in: FragmentInput, data: FragmentData) -> vec4<f32> {
    let world_normal = data.world_normal;

    let slope = 1.0 - world_normal.y;
    let height = in.world_position.y / config.height;

    var layer = 0.0;

    if (slope > 0.05) {
        layer = 1.0;
    }
    if (height > 0.6 && slope < 0.2) {
        layer = 2.0;
    }

    // Sample your custom material based on the layer.
    let uv = in.local_position / 100.0f;

    #ifndef ALBEDO
    let color = textureSample(array_texture, array_sampler, uv, i32(layer));
    #else
    let color = vec4<f32>(0.5);
    #endif

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

    return pbr(pbr_input);
}

// The default fragment entry point, which blends the terrain data at the fringe between two lods.
#import bevy_terrain::fragment