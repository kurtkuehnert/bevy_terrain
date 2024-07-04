#define_import_path bevy_terrain::debug

#import bevy_terrain::types::{Coordinate, NodeLookup, Blend, Tile}
#import bevy_terrain::bindings::{config, quadtree, view_config, tiles, attachments, origins, terrain_model_approximation}
#import bevy_terrain::functions::{lookup_best, approximate_view_distance, compute_morph, compute_blend, tile_count, quadtree_lod, inside_square, node_count, node_coordinate, coordinate_from_local_position, compute_subdivision_offset}
#import bevy_pbr::mesh_view_bindings::view

fn index_color(index: u32) -> vec4<f32> {
    var COLOR_ARRAY = array(
        vec4(1.0, 0.0, 0.0, 1.0),
        vec4(0.0, 1.0, 0.0, 1.0),
        vec4(0.0, 0.0, 1.0, 1.0),
        vec4(1.0, 1.0, 0.0, 1.0),
        vec4(1.0, 0.0, 1.0, 1.0),
        vec4(0.0, 1.0, 1.0, 1.0),
    );

    return mix(COLOR_ARRAY[index % 6u], vec4<f32>(0.6), 0.2);
}

fn quadtree_outlines(offset: vec2<f32>) -> f32 {
    let thickness = 0.015;

    return 1.0 - inside_square(offset, vec2<f32>(thickness), 1.0 - 2.0 * thickness);
}

fn show_tiles(tile: Tile, offset: vec2<f32>) -> vec4<f32> {
    let view_distance  = approximate_view_distance(tile, offset, view.world_position);
    let morph          = compute_morph(view_distance, tile.lod, offset);
    let target_lod     = log2(view_config.morph_distance / view_distance);

    var color        = select(index_color(tile.lod),     mix(index_color(tile.lod),     vec4(0.0), 0.5), (tile.xy.x + tile.xy.y) % 2u == 0u);
    let parent_color = select(index_color(tile.lod - 1), mix(index_color(tile.lod - 1), vec4(0.0), 0.5), ((tile.xy.x >> 1) + (tile.xy.y >> 1)) % 2u == 0u);
    color            = mix(color, parent_color, morph.ratio);


    if (distance(offset, compute_subdivision_offset(tile)) < 0.1) {
        color = mix(index_color(tile.lod + 1), vec4(0.0), 0.7);
    }

    if (fract(target_lod) < 0.01 && target_lod >= 1.0) {
        color = mix(color, vec4<f32>(0.0), 0.8);
    }

#ifdef SPHERICAL
    color = mix(color, index_color(tile.side), 0.3);
#endif

    if (max(0.0, target_lod) < f32(tile.lod) - 1.0 + view_config.morph_range || floor(target_lod) > f32(tile.lod)) {
        color = vec4<f32>(1.0, 0.0, 0.0, 1.0); // The view_distance and morph range are not sufficient.
    }

    return color;
}

fn show_quadtree(tile: Tile, offset: vec2<f32>) -> vec4<f32> {
    let lookup = lookup_best(tile, offset);

    let view_distance  = approximate_view_distance(tile, offset, view.world_position);
    let target_lod     = log2(view_config.blend_distance / view_distance);

    var color = index_color(lookup.atlas_lod);
    color     = mix(color, 0.1 * color, quadtree_outlines(lookup.atlas_uv));
    // color     = mix(color, vec4<f32>(0.0), quadtree_outlines(lookup.quadtree_uv));

    if (fract(target_lod) < 0.01 && target_lod >= 1.0) {
        color = mix(index_color(u32(target_lod)), vec4<f32>(0.0), 0.8);
    }

    return color;
}

fn show_lod(blend: Blend, lookup: NodeLookup) -> vec4<f32> {
    var color        = index_color(lookup.lod);
    let parent_color = index_color(lookup.lod - 1);
    color            = select(color, mix(color, parent_color, blend.ratio), blend.lod == lookup.lod);
    color            = mix(color, 0.1 * color, quadtree_outlines(lookup.uv));

    if (blend.ratio > 0.9 && blend.lod == lookup.lod) {   color = mix(color, vec4<f32>(0.0), 0.8); }

    // color = index_color(lookup.index);

    return color;
}

fn show_pixels(lookup: NodeLookup) -> vec4<f32> {
    let pixel_size = 4.0;
    let pixel_coordinate = lookup.uv * f32(attachments[0].size) / pixel_size;

    let is_even = (u32(pixel_coordinate.x) + u32(pixel_coordinate.y)) % 2u == 0u;

    if (is_even) { return vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else {         return vec4<f32>(0.1, 0.1, 0.1, 1.0); }
}
