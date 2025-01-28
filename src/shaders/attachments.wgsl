#define_import_path bevy_terrain::attachments

#import bevy_terrain::types::{AtlasTile, TangentSpace}
#import bevy_terrain::bindings::{terrain, terrain_view, terrain_sampler, attachments, height_attachment}
#import bevy_terrain::functions::tile_count

fn sample_height(tile: AtlasTile) -> f32 {
    let attachment = attachments.height;
    let uv         = tile.coordinate.uv * attachment.scale + attachment.offset;

#ifdef FRAGMENT
#ifdef SAMPLE_GRAD
    let height = textureSampleGrad(height_attachment, terrain_sampler, uv, tile.index, tile.coordinate.uv_dx, tile.coordinate.uv_dy).x;
#else
    let height = textureSampleLevel(height_attachment, terrain_sampler, uv, tile.index, 0.0).x;
#endif
#else
    let height = textureSampleLevel(height_attachment, terrain_sampler, uv, tile.index, 0.0).x;
#endif

    return terrain.height_scale * height;
}

fn sample_height_mask(tile: AtlasTile) -> bool {
    let attachment = attachments.height;
    let uv         = tile.coordinate.uv * attachment.scale + attachment.offset;
    let raw_height = textureGather(0, height_attachment, terrain_sampler, uv, tile.index);
    let mask       = bitcast<vec4<u32>>(raw_height) & vec4<u32>(1);

    return any(mask == vec4<u32>(0));
}

#ifdef FRAGMENT
fn sample_surface_gradient(tile: AtlasTile, tangent_space: TangentSpace) -> vec3<f32> {
    let attachment = attachments.height;

    let uv = tile.coordinate.uv * attachment.scale + attachment.offset;
    let uv_dx = tile.coordinate.uv_dx * attachment.scale;
    let uv_dy = tile.coordinate.uv_dy * attachment.scale;

    // Todo: compute lod using texture sample intrinsic once available in wgsl (textureQueryLod equivalent)
    let lod   = max(0., log2(attachment.texture_size * max(length(tile.coordinate.uv_dx), length(tile.coordinate.uv_dy))));
    let e_lod = exp2(lod);
    let uv_u  = uv + vec2<f32>(e_lod / attachment.texture_size, 0.0);
    let uv_v  = uv + vec2<f32>(0.0, e_lod / attachment.texture_size);

    let height     = textureSampleGrad(height_attachment, terrain_sampler, uv, tile.index, uv_dx, uv_dy).x;
    let height_u   = textureSampleGrad(height_attachment, terrain_sampler, uv_u, tile.index, uv_dx, uv_dy).x;
    let height_v   = textureSampleGrad(height_attachment, terrain_sampler, uv_v, tile.index, uv_dx, uv_dy).x;
    var height_duv = attachment.texture_size / e_lod * vec2<f32>(height_u - height, height_v - height);

    let start = 0.5;
    let end = 0.05;
    let ratio = saturate((lod - start) / (end - start));

    if (ratio > 0.0) {
        let coord = attachment.texture_size * uv - 0.5;
        let coord_floor = floor(coord);
        let center_uv = (coord_floor + 0.5) / attachment.texture_size;
        let height_TL = textureGather(0, height_attachment, terrain_sampler, center_uv, tile.index, vec2(-1, -1));
        let height_TR = textureGather(0, height_attachment, terrain_sampler, center_uv, tile.index, vec2( 1, -1));
        let height_BL = textureGather(0, height_attachment, terrain_sampler, center_uv, tile.index, vec2(-1,  1));
        let height_BR = textureGather(0, height_attachment, terrain_sampler, center_uv, tile.index, vec2( 1,  1));

        let height_matrix = mat4x4<f32>(height_TL.w, height_TL.z, height_TR.w, height_TR.z,
                                        height_TL.x, height_TL.y, height_TR.x, height_TR.y,
                                        height_BL.w, height_BL.z, height_BR.w, height_BR.z,
                                        height_BL.x, height_BL.y, height_BR.x, height_BR.y);

        let t  = saturate(coord - coord_floor);
        let A  = vec2<f32>(1.0 - t.x, t.x);
        let B  = vec2<f32>(1.0 - t.y, t.y);
        let X  = 0.25 * vec4<f32>(A.x, 2 * A.x + A.y, A.x + 2 * A.y, A.y);
        let Y  = 0.25 * vec4<f32>(B.x, 2 * B.x + B.y, B.x + 2 * B.y, B.y);
        let dX = 0.5 * vec4<f32>(-A.x, -A.y, A.x, A.y);
        let dY = 0.5 * vec4<f32>(-B.x, -B.y, B.x, B.y);
        let height_duv_upscaled = attachment.texture_size * vec2(dot(Y, dX * height_matrix), dot(dY, X * height_matrix));

        height_duv = mix(height_duv, height_duv_upscaled, ratio);
    }

    let height_dx = dot(height_duv, uv_dx);
    let height_dy = dot(height_duv, uv_dy);

    return terrain.height_scale * tangent_space.scale * (height_dx * tangent_space.tangent_x + height_dy * tangent_space.tangent_y);
}
#endif

fn compute_slope(world_normal: vec3<f32>, surface_gradient: vec3<f32>) -> f32 {
    let normal  = normalize(world_normal - surface_gradient);
    let cos_slope = min(dot(normal, world_normal), 1.0); // avoid artifacts
    return acos(cos_slope); // slope in radians
}
