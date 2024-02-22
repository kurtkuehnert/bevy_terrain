#define_import_path bevy_terrain::debug

#import bevy_terrain::types::{S2Coordinate}
#import bevy_terrain::bindings::{config, view_config, tiles, attachments}
#import bevy_terrain::functions::{compute_morph, compute_blend, quadtree_lod, inside_square, node_count, node_coordinate, coordinate_from_local_position}

fn index_color(index: u32) -> vec4<f32> {
    var COLOR_ARRAY = array<vec4<f32>, 6u>(
        vec4<f32>(1.0, 0.0, 0.0, 1.0),
        vec4<f32>(0.0, 1.0, 0.0, 1.0),
        vec4<f32>(0.0, 0.0, 1.0, 1.0),
        vec4<f32>(1.0, 1.0, 0.0, 1.0),
        vec4<f32>(1.0, 0.0, 1.0, 1.0),
        vec4<f32>(0.0, 1.0, 1.0, 1.0),
    );

    return COLOR_ARRAY[index % 6u];
}

fn show_tiles(coordinate: S2Coordinate, vertex_index: u32) -> vec4<f32> {
    let tile_index = vertex_index / view_config.vertices_per_tile;
    let tile = tiles.data[tile_index];

    var color: vec4<f32>;

    let is_even = u32((tile.st.x + tile.st.y) / tile.size) % 2u == 0u;

    if (is_even) { color = vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else {         color = vec4<f32>(0.1, 0.1, 0.1, 1.0); }

    let lod = u32(log2(tile.size));
    color = mix(color, index_color(lod), 0.5);

#ifdef MESH_MORPH
    let morph = compute_morph(tile, coordinate.st);
    color = mix(color, vec4<f32>(1.0), 0.3 * morph.ratio);
#endif

#ifdef SPHERICAL
    color = mix(color, index_color(tile.side), 0.5);
#endif

    return color;
}

fn show_lod(coordinate: S2Coordinate, view_distance: f32, lod: u32) -> vec4<f32> {
#ifdef QUADTREE_LOD
    let is_outline = quadtree_outlines(coordinate, lod);
    var color = mix(index_color(lod), vec4<f32>(0.0), is_outline);
#else
    let blend = compute_blend(view_distance);
    let is_outline = quadtree_outlines(coordinate, blend.lod);
    var color = mix(index_color(blend.lod), vec4<f32>(1.0), blend.ratio);
    color = mix(color, 0.1 * color, is_outline);
#endif

    return color;
}

fn quadtree_outlines(coordinate: S2Coordinate, lod: u32) -> f32 {
    let node_coordinate = node_coordinate(coordinate, lod) % 1.0;

    let thickness = 0.03;
    let outer = inside_square(node_coordinate, vec2<f32>(0.0)            , 1.0);
    let inner = inside_square(node_coordinate, vec2<f32>(0.0) + thickness, 1.0 - 2.0 * thickness);

    return outer - inner;
}

fn show_quadtree(coordinate: S2Coordinate) -> vec4<f32> {
    let lod = quadtree_lod(coordinate);
    let is_outline = quadtree_outlines(coordinate, lod);

    return mix(index_color(lod), vec4<f32>(0.0), is_outline);
}

fn show_pixels(coordinate: S2Coordinate, lod: u32) -> vec4<f32> {
    let pixel_size = 4.0;
    let pixels_per_side = attachments.data[0].size * f32(node_count(lod));
    let pixel_coordinate = coordinate.st * f32(pixels_per_side) / pixel_size;

    let is_even = (u32(pixel_coordinate.x) + u32(pixel_coordinate.y)) % 2u == 0u;

    if (is_even) { return vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else {         return vec4<f32>(0.1, 0.1, 0.1, 1.0); }
}
