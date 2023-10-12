#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Fragment, Blend, NodeLookup
#import bevy_terrain::vertex vertex_fn
#import bevy_terrain::bindings config, atlas_sampler
#import bevy_terrain::functions calculate_normal, calculate_blend, lookup_node
#import bevy_terrain::debug show_lod
#import bevy_terrain::attachments height_atlas, HEIGHT_SIZE, HEIGHT_SCALE, HEIGHT_OFFSET, albedo_atlas, ALBEDO_SIZE, ALBEDO_SCALE, ALBEDO_OFFSET
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::pbr_functions PbrInput, pbr_input_new, calculate_view, pbr

// Customize your material data here.
@group(3) @binding(0)
var array_texture: texture_2d_array<f32>;
@group(3) @binding(1)
var array_sampler: sampler;

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    return vertex_fn(in);
}

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
    let atlas_ddx = ddx / f32(1u << atlas_lod);
    let atlas_ddy = ddy / f32(1u << atlas_lod);

    // Adjust the uvs and deltas for your attachments.
    let height_coords = atlas_coords * HEIGHT_SCALE + HEIGHT_OFFSET;
    let height_ddx = atlas_ddx / HEIGHT_SIZE;
    let height_ddy = atlas_ddy / HEIGHT_SIZE;
    let albedo_coords = atlas_coords * ALBEDO_SCALE + ALBEDO_OFFSET;
    let albedo_ddx = atlas_ddx / ALBEDO_SIZE;
    let albedo_ddy = atlas_ddy / ALBEDO_SIZE;

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

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    let ddx   = dpdx(input.local_position);
    let ddy   = dpdy(input.local_position);
    let blend = calculate_blend(input.world_position);

    let lookup = lookup_node(blend.lod, input.local_position);
    var data   = lookup_fragment_data(input, lookup, ddx, ddy);

    if (blend.ratio < 1.0) {
        let lookup2 = lookup_node(blend.lod + 1u, input.local_position);
        let data2   = lookup_fragment_data(input, lookup2, ddx, ddy);
        data        = blend_fragment_data(data, data2, blend.ratio);
    }

    let fragment = process_fragment(input, data);

    if (fragment.do_discard) {
        discard;
    }

    return FragmentOutput(fragment.color);
}