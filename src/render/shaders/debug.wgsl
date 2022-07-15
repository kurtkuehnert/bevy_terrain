#define_import_path bevy_terrain::debug

fn lod_color(lod: u32) -> vec4<f32> {
    if (lod % 6u == 0u) {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if (lod % 6u == 1u) {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 2u) {
        return vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 3u) {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 4u) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 5u) {
        return vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    return vec4<f32>(0.0);
}

fn show_tiles(tile: Tile, local_position: vec2<f32>, tile_lod: u32) -> vec4<f32> {
    var color: vec4<f32>;

    if ((tile.coords.x + tile.coords.y) % 2u == 0u) {
        color = vec4<f32>(0.5, 0.5, 0.5, 1.0);
    }
    else {
        color = vec4<f32>(0.1, 0.1, 0.1, 1.0);
    }

    color = mix(color, lod_color(tile_lod), 0.5);

    if (tile.padding == 1u) {
        color = color * 10.0;
    }
    if (tile.padding == 2u) {
        color = color * 0.1;
    }

#ifdef MESH_MORPH
    let morph = calculate_morph(local_position, tile);
    color = color + vec4<f32>(1.0, 1.0, 1.0, 1.0) * morph;
#endif

    return color;
}

fn show_lod(lod: u32, world_position: vec3<f32>) -> vec4<f32> {
    var color = lod_color(lod);

    for (var i = 0u; i < config.lod_count; i = i + 1u) {
        let viewer_distance = distance(view.world_position.xyz, world_position);
        let circle = f32(1u << i) * view_config.view_distance;

        if (viewer_distance < circle && circle - f32(2 << i) < viewer_distance) {
            color = lod_color(i) * 10.0;
        }

#ifndef CIRCULAR_LOD
        let node_size = node_size(i);
        let grid_position = floor(view.world_position.xz / node_size + 0.5 - f32(view_config.node_count >> 1u)) * node_size;
        let grid_size = node_size * f32(view_config.node_count);
        let thickness = f32(4u << i);

        let grid_outer = step(grid_position, world_position.xz) * step(world_position.xz, grid_position + grid_size);
        let grid_inner = step(grid_position + thickness, world_position.xz) * step(world_position.xz, grid_position + grid_size - thickness);
        let outline = grid_outer.x * grid_outer.y - grid_inner.x * grid_inner.y;

        color = mix(color, lod_color(i) * 10.0, outline);
#endif
    }

    return color;
}
