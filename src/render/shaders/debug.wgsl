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

fn show_patches(patch: Patch, vertex_position: vec2<u32>) -> vec4<f32> {
    var color: vec4<f32>;

    if ((patch.x + patch.y) / config.patch_size % 2u == 0u) {
        color = vec4<f32>(0.5);
    }
    else {
        color = vec4<f32>(0.1);
    }

    let rim = config.patch_size >> 2u;

    if (vertex_position.x < rim && (patch.stitch & 1u) != 0u ||
        vertex_position.y < rim && (patch.stitch & 2u) != 0u ||
        vertex_position.x > config.patch_size - rim && (patch.stitch & 4u) != 0u ||
        vertex_position.y > config.patch_size - rim && (patch.stitch & 8u) != 0u) {
        color = vec4<f32>(-10.0);
    }

    return color;
}

fn show_lod(lod: u32, world_position: vec2<f32>) -> vec4<f32> {
    var color: vec4<f32>;

    color = lod_color(lod);

    for (var i = 0u; i < config.lod_count; i = i + 1u) {
        let node_size = node_size(i);
        let center = vec2<f32>(round(view.world_position.xz / node_size)) * node_size;

        let size = node_size * 4.0;
        let thickness = f32(4u << i);

        let hv1 = step(center - vec2<f32>(size), world_position) * step(world_position, center + vec2<f32>(size));
        let hv2 = step(center - vec2<f32>(size - thickness), world_position) * step(world_position, center + vec2<f32>(size - thickness));
        let onOff = hv1.x * hv1.y - hv2.x * hv2.y;

        color = mix(color, lod_color(i) * 4.0, onOff);
    }

    let distance = distance(view.world_position.xz, world_position);
    let circle = f32(2u << lod) * config.view_distance;

    if (distance < circle && circle - f32(4 << lod) < distance) {
        color = color * 4.0;
    }

    return color;
}