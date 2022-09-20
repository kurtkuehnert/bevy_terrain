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
// It will be called once in the fragment shader with the blended fragment data.
// fn color_fragment(in: FragmentInput, data: FragmentData) -> vec4<f32>;

// The default fragment entry point, which blends the terrain data at the fringe between two lods.
@fragment
fn fragment(fragment: FragmentInput) -> FragmentOutput {
    if (fragment.local_position.x < 2.0 || fragment.local_position.x > f32(config.terrain_size) - 2.0 ||
        fragment.local_position.y < 2.0 || fragment.local_position.y > f32(config.terrain_size) - 2.0) {
        discard;
    }

    if (fragment.world_position.y == 0.0) {
        discard;
    }

    let blend = calculate_blend(fragment.world_position.xyz, view_config.fragment_blend);

    let lookup = atlas_lookup(blend.lod, fragment.local_position);
    var fragment_data = lookup_fragment_data(fragment, lookup);

    if (blend.ratio < 1.0) {
        let lookup2 = atlas_lookup(blend.lod + 1u, fragment.local_position);
        let fragment_data2 = lookup_fragment_data(fragment, lookup2);

        fragment_data = blend_fragment_data(fragment_data, fragment_data2, blend.ratio);
    }

    var color = fragment_color(fragment, fragment_data);

    color = mix(fragment.color, color, 0.8);

    return FragmentOutput(color);
}