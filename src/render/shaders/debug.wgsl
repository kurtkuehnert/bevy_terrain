#define_import_path bevy_terrain::debug

#import bevy_terrain::types Tile, S2Coordinate
#import bevy_terrain::bindings config, view_config
#import bevy_terrain::functions morph, blend, quadtree_lod, inside_rect, node_coordinate, s2_from_world_position
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

fn show_tiles(tile: Tile, world_position: vec4<f32>) -> vec4<f32> {
    var color: vec4<f32>;

    let is_even = u32((tile.uv.x + tile.uv.y) / tile.size) % 2u == 0u;

    if (is_even) { color = vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    else {         color = vec4<f32>(0.1, 0.1, 0.1, 1.0); }

    let lod = u32(log2(1.0 / tile.size));
    color = mix(color, index_color(lod), 0.5);

#ifdef MESH_MORPH
    let morph = morph(tile, world_position);
    color = mix(color, vec4<f32>(1.0), 0.3 * morph);
#endif

    return color;
}

fn show_lod(world_position: vec4<f32>) -> vec4<f32> {
    let blend = blend(world_position);
    let lod = blend.lod;

    var color = index_color(lod);

    let viewer_distance = distance(view.world_position.xyz, world_position.xyz);

    for (var lod = 0u; lod < config.lod_count; lod = lod + 1u) {
        let circle = f32(2u << lod) * 2.0 *  view_config.view_distance;
        let thickness = 0.5 * f32(1u << lod);

        if (viewer_distance < circle && circle - thickness < viewer_distance) {
            color = index_color(lod) * 0.1;
        }
    }

    color.a = 1.0;

    return color;
}

fn quadtree_outlines(world_position: vec4<f32>, lod: u32) -> f32 {
    let s2 = s2_from_world_position(world_position);
    let coordinate = node_coordinate(s2.st, lod) % 1.0;

    let thickness = 0.03;
    let outer = inside_rect(coordinate, vec2<f32>(0.0)            , 1.0);
    let inner = inside_rect(coordinate, vec2<f32>(0.0) + thickness, 1.0 - thickness);

    return outer - inner;
}

fn show_quadtree(world_position: vec4<f32>) -> vec4<f32> {
    let lod = quadtree_lod(world_position);
    let is_outline = quadtree_outlines(world_position, lod);

    return mix(index_color(lod), vec4<f32>(0.0), is_outline);
}
