#define_import_path bevy_terrain::fragment

#import bevy_terrain::types FragmentInput, FragmentOutput, Fragment, Blend, NodeLookup
#import bevy_terrain::bindings config
#import bevy_terrain::functions calculate_normal, calculate_blend, lookup_node
#import bevy_terrain::debug show_lod
#import bevy_terrain::attachments height_atlas, HEIGHT_SCALE, HEIGHT_OFFSET
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::pbr_functions PbrInput, pbr_input_new, calculate_view, pbr

struct FragmentData {
    world_normal: vec3<f32>,
    debug_color: vec4<f32>,
}

fn lookup_fragment_data(input: FragmentInput, lookup: NodeLookup, ddx: vec2<f32>, ddy: vec2<f32>) -> FragmentData {
    let atlas_lod = lookup.atlas_lod;
    let atlas_index = lookup.atlas_index;
    let atlas_coords = lookup.atlas_coords;
    let atlas_ddx = ddx / f32(1u << atlas_lod);
    let atlas_ddy = ddy / f32(1u << atlas_lod);

    let height_coords = atlas_coords * HEIGHT_SCALE + HEIGHT_OFFSET;
    let height_ddx = atlas_ddx / 512.0;
    let height_ddy = atlas_ddy / 512.0;

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

// The terrain data required by your `fragment_color` function.
// This data will be fetched from the atlases by means of the `AtlasLookup`.
// To smoothen the transition between different lods the fragment data will be blended at the fringe between them.
// struct FragmentData;

// Lookup the terrain data required by your `fragment_color` function.
// This will happen once or twice (lod fringe).
// fn lookup_fragment_data(in: FragmentInput, lookup: AtlasLookup) -> FragmentData;

// Blend the terrain data on the fringe between two lods.
// fn blend_fragment_data(data1: FragmentData, data2: FragmentData, blend_ratio: f32) -> FragmentData;

// The function that evaluates the color of the fragment.
// It will be called once in the fragment shader with the fragment input and the
// blended fragment data.
// fn process_fragment(input: FragmentInput, data: FragmentData) -> Fragment;

// The default fragment entry point, which blends the terrain data at the fringe between two lods.
@fragment
fn fragment_fn(input: FragmentInput) -> FragmentOutput {
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