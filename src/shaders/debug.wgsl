#define_import_path bevy_terrain::debug

#import bevy_terrain::types::{Coordinate, NodeLookup, Blend, Tile}
#import bevy_terrain::bindings::{config, quadtree, view_config, tiles, attachments, origins, terrain_model_approximation}
#import bevy_terrain::functions::{approximate_view_distance, compute_morph, compute_blend, tile_count, quadtree_lod, inside_square, node_count, node_coordinate, coordinate_from_local_position, compute_subdivision_offsets}
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
    let thickness = 0.02;

    return 1.0 - inside_square(offset, vec2<f32>(thickness), 1.0 - 2.0 * thickness);
}

fn show_tiles(tile: Tile, offset: vec2<f32>) -> vec4<f32> {
    let view_distance  = approximate_view_distance(tile, offset, view.world_position);
    let morph          = compute_morph(view_distance, tile.lod, offset);

    let lod_difference = (tile.lod - morph.lod);
    let xy = vec2<u32>(tile.xy.x >> lod_difference, tile.xy.y >> lod_difference);

    var color        = select(index_color(morph.lod),     mix(index_color(morph.lod),     vec4(0.0), 0.5), (xy.x + xy.y) % 2u == 0u);
    let parent_color = select(index_color(morph.lod - 1), mix(index_color(morph.lod - 1), vec4(0.0), 0.5), ((xy.x >> 1) + (xy.y >> 1)) % 2u == 0u);
    color            = mix(color, parent_color, morph.ratio);

    let target_lod = log2(view_config.morph_distance / view_distance);

    // color = index_color(tile.lod);
    // color = index_color(u32(target_lod - 0.3));

    if (lod_difference != 0) {
        color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }

    var offsets = compute_subdivision_offsets(tile);

    for (var i = 0; i < 5; i += 1) {
        if (length(offset - offsets[i]) < view_config.morph_distance / 100.0 / config.scale) {
            color = mix(color, vec4<f32>(0.0), 0.9);
        }
    }

#ifdef SPHERICAL
    color = mix(color, index_color(tile.side), 0.3);
#endif

    if (tile.lod < u32(target_lod)) { color = vec4<f32>(1.0, 0.0, 0.0, 1.0); }
    if (fract(target_lod) < 0.01) {   color = mix(color, vec4<f32>(0.0), 0.6); }

    return color;
}

fn show_lod(blend: Blend, lookup: NodeLookup) -> vec4<f32> {
    var color        = index_color(lookup.lod);
    let parent_color = index_color(lookup.lod - 1);
    color            = select(color, mix(color, parent_color, blend.ratio), blend.lod == lookup.lod);
    color            = mix(color, 0.1 * color, quadtree_outlines(lookup.uv));

    if (blend.ratio > 0.9) {   color = mix(color, vec4<f32>(0.0), 0.8); }

    return color;
}

fn show_quadtree(tile: Tile, offset: vec2<f32>) -> vec4<f32> {
    var quadtree_lod = 0u;
    var node_xy: vec2<i32>;
    var node_uv: vec2<f32>;
    var quadtree_uv: vec2<f32>;

    for (; quadtree_lod < config.lod_count; quadtree_lod += 1u) {
        let origin_xy = vec2<i32>(origins[tile.side * config.lod_count + quadtree_lod]);

        let lod_difference = i32(tile.lod) - i32(quadtree_lod);

        var new_node_xy: vec2<i32>;
        var new_node_uv: vec2<f32>;

        if (lod_difference < 0) {
            let size = 1u << u32(-lod_difference);
            let scaled_offset = offset * f32(size);
            new_node_xy = vec2<i32>(tile.xy * size) + vec2<i32>(scaled_offset);
            new_node_uv = scaled_offset % 1.0;
        } else {
            let size = 1u << u32(lod_difference);
            new_node_xy = vec2<i32>(tile.xy / size);
            new_node_uv = (vec2<f32>(tile.xy % size) + offset) / f32(size);
        }

        let new_quadtree_uv = (vec2<f32>(new_node_xy - origin_xy) + new_node_uv) / f32(view_config.quadtree_size);

        if (any(new_quadtree_uv < vec2<f32>(0.0)) || any(new_quadtree_uv > vec2<f32>(1.0))) {
            quadtree_lod -= 1u;
            break;
        }

        quadtree_uv = new_quadtree_uv;
        node_xy     = new_node_xy;
        node_uv     = new_node_uv;
    }

    let quadtree_side  = tile.side;
    let quadtree_xy    = vec2<u32>(node_xy) % view_config.quadtree_size;
    let quadtree_index = (((                            quadtree_side) *
                            config.lod_count          + quadtree_lod ) *
                            view_config.quadtree_size + quadtree_xy.x) *
                            view_config.quadtree_size + quadtree_xy.y;
    let quadtree_entry = quadtree[quadtree_index];

    quadtree_lod = quadtree_entry.atlas_lod;

    // Todo: use this node as quadtree lod again

    var color = index_color(quadtree_lod);
    // color     = vec4<f32>(node_uv, 0.0, 1.0);
    // color     = vec4<f32>(quadtree_uv, 0.0, 1.0);
    color     = mix(color, 0.1 * color, quadtree_outlines(node_uv));
    color     = mix(color, 0.5 * vec4<f32>(1.0), quadtree_outlines(quadtree_uv));

    return color;
}

fn show_pixels(lookup: NodeLookup) -> vec4<f32> {
    let pixel_size = 4.0;
    let pixel_coordinate = lookup.uv * f32(attachments[0].size) / pixel_size;

    let is_even = (u32(pixel_coordinate.x) + u32(pixel_coordinate.y)) % 2u == 0u;

    if (is_even) { return vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else {         return vec4<f32>(0.1, 0.1, 0.1, 1.0); }
}
