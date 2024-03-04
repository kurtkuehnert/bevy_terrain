#define_import_path bevy_terrain::debug

#import bevy_terrain::types::{UVCoordinate, Blend}
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

fn quadtree_outlines(coordinate: UVCoordinate, lod: u32) -> f32 {
    let node_coordinate = node_coordinate(coordinate, lod) % 1.0;
    let thickness       = 0.02;
    let outer           = inside_square(node_coordinate, vec2<f32>(-0.001)         , 1.001);
    let inner           = inside_square(node_coordinate, vec2<f32>(0.0) + thickness, 1.0 - 2.0 * thickness);

    return outer - inner;
}

fn show_tiles(view_distance: f32, vertex_index: u32) -> vec4<f32> {
    let tile_index = vertex_index / view_config.vertices_per_tile;
    let tile = tiles[tile_index];

    var color: vec4<f32>;

    if ((tile.xy.x + tile.xy.y) % 2u == 0u) { color = vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else                                    { color = vec4<f32>(0.1, 0.1, 0.1, 1.0); }

    color = mix(color, index_color(tile.lod), 0.5);

#ifdef MORPH
    let morph_ratio = compute_morph(view_distance, tile.lod);
    color           = mix(color, vec4<f32>(1.0), 0.5 * morph_ratio);
#endif

#ifdef SPHERICAL
    color = mix(color, index_color(tile.side), 0.5);
#endif

    return color;
}

fn show_lod(coordinate: UVCoordinate, view_distance: f32, lod: u32) -> vec4<f32> {
#ifdef QUADTREE_LOD
    let blend = Blend(lod, 0.0);
#else
    let blend = compute_blend(view_distance);
#endif

    let is_outline = quadtree_outlines(coordinate, blend.lod);
    var color      = index_color(blend.lod);
    color          = mix(color, vec4<f32>(1.0), 0.5 * blend.ratio);
    color          = mix(color, 0.1 * color, is_outline);

    return color;
}

fn show_quadtree(coordinate: UVCoordinate) -> vec4<f32> {
    let lod        = quadtree_lod(coordinate);
    let is_outline = quadtree_outlines(coordinate, lod);
    var color      = index_color(lod);
    color          = mix(color, 0.1 * color, is_outline);

    return color;
}

fn show_pixels(coordinate: UVCoordinate, lod: u32) -> vec4<f32> {
    let pixel_size = 4.0;
    let pixels_per_side = attachments[0].size * node_count(lod);
    let pixel_coordinate = coordinate.uv * f32(pixels_per_side) / pixel_size;

    let is_even = (u32(pixel_coordinate.x) + u32(pixel_coordinate.y)) % 2u == 0u;

    if (is_even) { return vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else {         return vec4<f32>(0.1, 0.1, 0.1, 1.0); }
}
