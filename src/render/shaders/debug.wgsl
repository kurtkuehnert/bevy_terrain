#define_import_path bevy_terrain::debug

fn lod_color(lod: u32) -> vec4<f32> {
    if (lod == 0u) {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if (lod == 1u) {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    if (lod == 2u) {
        return vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }
    if (lod == 3u) {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
    if (lod == 4u) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if (lod == 5u) {
        return vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    return vec4<f32>(0.0);
}

fn show_patches(patch: Patch, local_position: vec2<f32>) -> vec4<f32> {
    var color: vec4<f32>;

    if ((patch.coords.x + patch.coords.y) / config.patch_size % 2u == 0u) {
        color = vec4<f32>(0.5);
    }
    else {
        color = vec4<f32>(0.1);
    }

#ifdef MESH_MORPH
    let viewer_distance = distance(local_position, view.world_position.xz);
    let morph_distance = f32(patch.size << 1u) * config.view_distance;
    let morph = clamp(1.0 - (1.0 - viewer_distance / morph_distance) / morph_blend, 0.0, 1.0);

    color = mix(color, vec4<f32>(1.0, 0.0, 0.0, 1.0), morph);
#endif

    return color;
}

fn show_lod(lod: u32, world_position: vec2<f32>) -> vec4<f32> {
    var color: vec4<f32>;

    color = lod_color(lod);

    for (var i = 0u; i < config.lod_count; i = i + 1u) {
        let node_size = node_size(i);
        let grid_position = floor(view.world_position.xz / node_size + 0.5 - f32(config.node_count) / 2.0) * node_size;
        let grid_size = node_size * f32(config.node_count);
        let thickness = f32(4u << i);

        let grid_outer = step(grid_position, world_position) * step(world_position, grid_position + grid_size);
        let grid_inner = step(grid_position + thickness, world_position) * step(world_position, grid_position + grid_size - thickness);
        let outline = grid_outer.x * grid_outer.y - grid_inner.x * grid_inner.y;

        color = mix(color, lod_color(i) * 4.0, outline);
    }

    let distance = distance(view.world_position.xz, world_position);
    let circle = f32(2u << lod) * config.view_distance;

    if (distance < circle && circle - f32(4 << lod) < distance) {
        color = color * 100.0;
    }

    return color;
}
