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

// The function that evaluates the color of the fragment.
// It will be called in the fragment shader and blended between different lods.
fn color_fragment(
    in: FragmentInput,
    lod: u32,
    atlas_index: i32,
    atlas_coords: vec2<f32>
) -> vec4<f32> {
    // Adjust the uvs for your attachments.
    let height_coords = atlas_coords * config.height_scale + config.height_offset;

    // Calculate the normal from the heightmap.
    let world_normal = calculate_normal(height_coords, atlas_index, lod);

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

// Import the default vertex and fragment entry points.
#import bevy_terrain::entry_points