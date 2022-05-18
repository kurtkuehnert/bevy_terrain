#define_import_path bevy_terrain::debug

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

    if (lod == 0u) {
        color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if (lod == 1u) {
        color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    if (lod == 2u) {
        color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }
    if (lod == 3u) {
        color = vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
    if (lod == 4u) {
        color = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if (lod == 5u) {
        color = vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    let distance = distance(view.world_position.xz, world_position);
    let circle = f32(2u << lod) * config.view_distance;

    if (distance < circle && circle - f32(4 << lod) < distance) {
        color = color * 4.0;
    }

    return color;
}