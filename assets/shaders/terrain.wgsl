#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_types
#import bevy_terrain::config
#import bevy_terrain::patch

// Todo: make these configurable
let height_scale :  f32 = 0.96969696969; // 128 / 132
let height_offset:  f32 = 0.01515151515; //   2 / 132
let albedo_scale :  f32 = 0.9968847352;  // 640 / 642
let albedo_offset:  f32 = 0.00155763239; //   1 / 642
let morph_blend:    f32 = 0.2;
let vertex_blend:   f32 = 0.3;
let fragment_blend: f32 = 0.3;

// terrain view bindings
[[group(1), binding(0)]]
var<uniform> view_config: TerrainViewConfig;
[[group(1), binding(1)]]
var quadtree: texture_2d_array<u32>;
[[group(1), binding(2)]]
var<storage> patches: PatchList;

// terrain bindings
[[group(2), binding(0)]]
var<uniform> config: TerrainConfig;
[[group(2), binding(1)]]
var filter_sampler: sampler;
[[group(2), binding(2)]]
var height_atlas: texture_2d_array<f32>;
#ifdef ALBEDO
[[group(2), binding(3)]]
var albedo_atlas: texture_2d_array<f32>;
#endif

// mesh bindings
[[group(3), binding(0)]]
var<uniform> mesh: Mesh;

#import bevy_pbr::pbr_types
#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::pbr_functions

#import bevy_terrain::utils
#import bevy_terrain::debug

fn height_vertex(atlas_index: i32, atlas_coords: vec2<f32>) -> f32 {
    let height_coords = atlas_coords * height_scale + height_offset;
    return config.height * textureSampleLevel(height_atlas, filter_sampler, height_coords, atlas_index, 0.0).x;
}

fn color_fragment(
    in: FragmentInput,
    lod: u32,
    atlas_index: i32,
    atlas_coords: vec2<f32>
) -> vec4<f32> {
    var color = vec4<f32>(0.0);

    let height_coords = atlas_coords * height_scale + height_offset;
    let albedo_coords = atlas_coords * albedo_scale + albedo_offset;

    #ifndef BRIGHT
        color = mix(color, vec4<f32>(1.0), 0.5);
    #endif

    #ifdef SHOW_LOD
        color = mix(color, show_lod(lod, in.world_position.xyz), 0.4);
    #endif

    #ifdef ALBEDO
        color = mix(color, textureSample(albedo_atlas, filter_sampler, albedo_coords, atlas_index), 0.5);
    #endif

    #ifdef SHOW_UV
        color = mix(color, vec4<f32>(atlas_coords.x, atlas_coords.y, 0.0, 1.0), 0.5);
    #endif

    #ifdef LIGHTING
        let world_normal = calculate_normal(height_coords, atlas_index, lod);

        let ambient = 0.1;
        let direction = normalize(vec3<f32>(3.0, 1.0, -2.0));
        let diffuse = max(dot(direction, world_normal), 0.0);
        color = color * (ambient + diffuse);

        // var pbr_input: PbrInput = pbr_input_new();
        // pbr_input.material.base_color = color;
        // pbr_input.frag_coord = in.frag_coord;
        // pbr_input.world_position = in.world_position;
        // pbr_input.world_normal = world_normal;
        // pbr_input.is_orthographic = view.projection[3].w == 1.0;
        // pbr_input.N = world_normal;
        // pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);
        // color = vec4<f32>(pbr_input.V, 1.0);
        // color = pbr(pbr_input);
    #endif

    return color;
}

[[stage(vertex)]]
fn vertex(vertex: VertexInput) -> VertexOutput {
    let patch_index = vertex.index / view_config.vertices_per_patch;
    let vertex_index = vertex.index % view_config.vertices_per_patch;

    let patch = patches.data[patch_index];
    let local_position = calculate_position(vertex_index, patch);

    let world_position = vec3<f32>(local_position.x, view_config.height_under_viewer, local_position.y);
    let blend = calculate_blend(world_position, vertex_blend);

    let lookup = atlas_lookup(blend.log_distance, local_position);
    var height = height_vertex(lookup.atlas_index, lookup.atlas_coords);

    if (blend.ratio < 1.0) {
        let lookup2 = atlas_lookup(blend.log_distance + 1.0, local_position);
        var height2 = height_vertex(lookup2.atlas_index, lookup2.atlas_coords);
        height = mix(height2, height, blend.ratio);
    }

    var output = vertex_output(local_position, height);

#ifdef SHOW_PATCHES
    output.color = show_patches(patch, local_position);
#endif

    return output;
}

[[stage(fragment)]]
fn fragment(fragment: FragmentInput) -> [[location(0)]] vec4<f32> {
    let blend = calculate_blend(fragment.world_position.xyz, fragment_blend);

    let lookup = atlas_lookup(blend.log_distance, fragment.local_position);
    var color = color_fragment(fragment, lookup.lod, lookup.atlas_index, lookup.atlas_coords);

    if (blend.ratio < 1.0) {
        let lookup2 = atlas_lookup(blend.log_distance + 1.0, fragment.local_position);
        let color2 = color_fragment(fragment, lookup2.lod, lookup2.atlas_index, lookup2.atlas_coords);
        color = mix(color2, color, blend.ratio);
    }

    return mix(fragment.color, color, 0.8);
}
