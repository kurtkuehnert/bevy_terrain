#define_import_path bevy_terrain::debug

#import bevy_terrain::vertex::{VertexInfo}
#import bevy_terrain::types::{Coordinate, NodeLookup, Blend}
#import bevy_terrain::bindings::{config, view_config, tiles, attachments}
#import bevy_terrain::functions::{compute_morph, compute_blend, quadtree_lod, inside_square, node_count, node_coordinate, coordinate_from_local_position, tile_size}

fn index_color(index: u32) -> vec4<f32> {
    var COLOR_ARRAY = array(
        vec4(1.0, 0.0, 0.0, 1.0),
        vec4(0.0, 1.0, 0.0, 1.0),
        vec4(0.0, 0.0, 1.0, 1.0),
        vec4(1.0, 1.0, 0.0, 1.0),
        vec4(1.0, 0.0, 1.0, 1.0),
        vec4(0.0, 1.0, 1.0, 1.0),
    );

    return COLOR_ARRAY[index % 6u];
}

fn quadtree_outlines(lookup: NodeLookup) -> f32 {
    let thickness = 0.02;

    return 1.0 - inside_square(lookup.uv, vec2<f32>(thickness), 1.0 - 2.0 * thickness);
}

fn show_tiles(info: ptr<function, VertexInfo>) -> vec4<f32> {
    var color: vec4<f32>;

    if (((*info).tile.xy.x + (*info).tile.xy.y) % 2u == 0u) { color = vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else                                                    { color = vec4<f32>(0.1, 0.1, 0.1, 1.0); }

    color = mix(color, index_color((*info).tile.lod), 0.5);

#ifdef MORPH
    let morph = compute_morph((*info).view_distance, (*info).tile.lod, vec2<f32>(0.0));
    color     = mix(color, vec4<f32>(1.0), 0.5 * morph.ratio);
#endif

#ifdef SPHERICAL
    color = mix(color, index_color((*info).tile.side), 0.5);
#endif

    return color;
}

fn show_lod(blend: Blend, lookup: NodeLookup) -> vec4<f32> {
    let is_outline = quadtree_outlines(lookup);
    var color      = index_color(lookup.lod);

    if (blend.lod == lookup.lod) {
        color          = mix(color, vec4<f32>(1.0), 0.5 * blend.ratio);
    }

    color          = mix(color, 0.1 * color, is_outline);

    return color;
}

fn show_quadtree(coordinate: Coordinate) -> vec4<f32> {
    let color = vec4<f32>(0.0);

    return color;
}

fn show_pixels(coordinate: Coordinate, lod: u32) -> vec4<f32> {
    let pixel_size = 1.0;
    let pixels_per_side = attachments[0].size * node_count(lod);
    let pixel_coordinate = coordinate.uv * f32(pixels_per_side) / pixel_size;

    let is_even = (u32(pixel_coordinate.x) + u32(pixel_coordinate.y)) % 2u == 0u;

    if (is_even) { return vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else {         return vec4<f32>(0.1, 0.1, 0.1, 1.0); }
}
