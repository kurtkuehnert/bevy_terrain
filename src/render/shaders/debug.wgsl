#define_import_path bevy_terrain::debug

#import bevy_terrain::bindings config, view_config, tiles
#import bevy_terrain::functions compute_morph, compute_blend, quadtree_lod, inside_square, node_coordinate, s2_from_local_position
#import bevy_pbr::mesh_view_bindings view

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

fn show_tiles(vertex_index: u32, world_position: vec4<f32>) -> vec4<f32> {
    let tile_index = vertex_index / view_config.vertices_per_tile;
    let tile = tiles.data[tile_index];

    var color: vec4<f32>;

    let is_even = u32((tile.uv.x + tile.uv.y) / tile.size) % 2u == 0u;

    if (is_even) { color = vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else {         color = vec4<f32>(0.1, 0.1, 0.1, 1.0); }

    let lod = u32(log2(1.0 / tile.size));
    color = mix(color, index_color(lod), 0.5);

#ifdef MESH_MORPH
    let morph = compute_morph(tile, world_position);
    color = mix(color, vec4<f32>(1.0), 0.3 * morph.ratio);
#endif

#ifdef SPHERICAL
    color = mix(color, index_color(tile.side), 0.5);
#endif

    return color;
}

fn show_lod(local_position: vec3<f32>, world_position: vec4<f32>, atlas_lod: u32) -> vec4<f32> {
#ifdef QUADTREE_LOD
    let is_outline = quadtree_outlines(local_position, atlas_lod);
    let color = mix(index_color(atlas_lod), vec4<f32>(0.0), is_outline);
#else
    let blend = compute_blend(world_position);
    let is_outline = quadtree_outlines(local_position, blend.lod);
    var color = mix(index_color(blend.lod), vec4<f32>(1.0), 1.0 - blend.ratio);
    color = mix(color, 0.1 * color, is_outline);
#endif

    return color;
}

fn quadtree_outlines(local_position: vec3<f32>, lod: u32) -> f32 {
    let s2 = s2_from_local_position(local_position);
    let coordinate = node_coordinate(s2.st, lod) % 1.0;

    let thickness = 0.03;
    let outer = inside_square(coordinate, vec2<f32>(0.0)            , 1.0);
    let inner = inside_square(coordinate, vec2<f32>(0.0) + thickness, 1.0 - thickness);

    return outer - inner;
}

fn show_quadtree(local_position: vec3<f32>) -> vec4<f32> {
    let lod = quadtree_lod(local_position);
    let is_outline = quadtree_outlines(local_position, lod);

    return mix(index_color(lod), vec4<f32>(0.0), is_outline);
}
