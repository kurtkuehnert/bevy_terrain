#define_import_path bevy_terrain::fragment

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