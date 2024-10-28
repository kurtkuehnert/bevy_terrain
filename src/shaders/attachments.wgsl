#define_import_path bevy_terrain::attachments

#import bevy_terrain::types::{AtlasTile, TangentSpace}
#import bevy_terrain::bindings::{terrain, terrain_sampler, attachments, attachment0, attachment1, attachment2}
#import bevy_terrain::functions::tile_count

fn attachment_uv(uv: vec2<f32>, attachment_index: u32) -> vec2<f32> {
    let attachment = attachments[attachment_index];
    return uv * attachment.scale + attachment.offset;
}

fn sample_attachment0(tile: AtlasTile) -> vec4<f32> {
    let uv = attachment_uv(tile.coordinate.uv, 0u);

#ifdef FRAGMENT
#ifdef SAMPLE_GRAD
    return textureSampleGrad(attachment0, terrain_sampler, uv, tile.index, tile.coordinate.uv_dx, tile.coordinate.uv_dy);
#else
    return textureSampleLevel(attachment0, terrain_sampler, uv, tile.index, 0.0);
#endif
#else
    return textureSampleLevel(attachment0, terrain_sampler, uv, tile.index, 0.0);
#endif
}

fn sample_attachment1(tile: AtlasTile) -> vec4<f32> {
    let uv = attachment_uv(tile.coordinate.uv, 1u);

#ifdef FRAGMENT
#ifdef SAMPLE_GRAD
    return textureSampleGrad(attachment1, terrain_sampler, uv, tile.index, tile.coordinate.uv_dx, tile.coordinate.uv_dy);
#else
    return textureSampleLevel(attachment1, terrain_sampler, uv, tile.index, 0.0);
#endif
#else
    return textureSampleLevel(attachment1, terrain_sampler, uv, tile.index, 0.0);
#endif
}

fn sample_attachment0_gather0(tile: AtlasTile) -> vec4<f32> {
    let uv = attachment_uv(tile.coordinate.uv, 0u);
    return textureGather(0, attachment0, terrain_sampler, uv, tile.index);
}

fn sample_height(tile: AtlasTile) -> f32 {
    let height = sample_attachment0(tile).x;

    return mix(terrain.min_height, terrain.max_height, height);
}

fn sample_height_mask(tile: AtlasTile) -> bool {
    let raw_height = sample_attachment0_gather0(tile);
    let mask = bitcast<vec4<u32>>(raw_height) & vec4<u32>(1);

    return any(mask == vec4<u32>(0));
}

#ifdef FRAGMENT
fn compute_tangent_space(world_position: vec4<f32>, world_normal: vec3<f32>) -> TangentSpace {
    // Todo: this should be the position before displacement (ellipsoid)
    let position    = world_position.xyz;
    let position_dx = dpdx(position);
    let position_dy = dpdy(position);

    let tangent_x = cross(position_dy, world_normal);
    let tangent_y = cross(world_normal, position_dx);
    let scale = 1.0 / dot(position_dx, tangent_x);

    return TangentSpace(tangent_x, tangent_y, scale);
}

fn sample_surface_gradient(tile: AtlasTile, tangent_space: TangentSpace) -> vec3<f32> {
    let texture_size = 512.0; // store the actual texture size in attachments as well
    let attachment = attachments[0];
    let uv = tile.coordinate.uv * attachment.scale + attachment.offset;
    let uv_dx = tile.coordinate.uv_dx * attachment.scale;
    let uv_dy = tile.coordinate.uv_dy * attachment.scale;

    let height   = textureSampleGrad(attachment0, terrain_sampler, uv, tile.index, uv_dx, uv_dy).x;

    var height_dx = 0.0;
    var height_dy = 0.0;

//    {  // massively imprecise, when the texture is magnified (derivative is piecewise constant)
//        height_dx  = dpdx(height);
//        height_dy  = dpdy(height);
//    }
//    { // still imprecise, similar to solution above, but sampling using exclicite texture samples
//        let height_x = textureSampleGrad(attachment0, terrain_sampler, uv + uv_dx, tile.index, uv_dx, uv_dy).x;
//        let height_y = textureSampleGrad(attachment0, terrain_sampler, uv + uv_dy, tile.index, uv_dx, uv_dy).x;
//
//        height_dx = height_x - height;
//        height_dy = height_y - height;
//    }
    {
        // Todo: compute lod using texture sample intrinsic once available in wgsl
        let lod = max(0., log2(attachment.size * max(length(tile.coordinate.uv_dx), length(tile.coordinate.uv_dy))));
        let e_lod = exp2(lod);
        let uv_u = uv + vec2<f32>(e_lod / attachment.size, 0.0);
        let uv_v = uv + vec2<f32>(0.0, e_lod / attachment.size);

        let height_u = textureSampleGrad(attachment0, terrain_sampler, uv_u, tile.index, uv_dx, uv_dy).x;
        let height_v = textureSampleGrad(attachment0, terrain_sampler, uv_v, tile.index, uv_dx, uv_dy).x;

        var height_duv = vec2<f32>(height_u - height, height_v - height) * attachment.size / e_lod;

        let start = 0.5;
        let end = 0.05;
        let ratio = saturate((lod - start) / (end - start));

        if (ratio > 0.0) {
            let coord = texture_size * uv - 0.5;
            let coord_floor = floor(coord);
            let center_uv = (coord_floor + 0.5) / texture_size;
            let height_TL = textureGather(0, attachment0, terrain_sampler, center_uv, tile.index, vec2(-1, -1));
            let height_TR = textureGather(0, attachment0, terrain_sampler, center_uv, tile.index, vec2( 1, -1));
            let height_BL = textureGather(0, attachment0, terrain_sampler, center_uv, tile.index, vec2(-1,  1));
            let height_BR = textureGather(0, attachment0, terrain_sampler, center_uv, tile.index, vec2( 1,  1));

            let height_matrix = mat4x4<f32>(height_TL.w, height_TL.z, height_TR.w, height_TR.z,
                                            height_TL.x, height_TL.y, height_TR.x, height_TR.y,
                                            height_BL.w, height_BL.z, height_BR.w, height_BR.z,
                                            height_BL.x, height_BL.y, height_BR.x, height_BR.y);

            let t = saturate(coord - coord_floor);
            let A = vec2<f32>(1.0 - t.x, t.x);
            let B = vec2<f32>(1.0 - t.y, t.y);
            let X = 0.25 * vec4<f32>(A.x, 2 * A.x + A.y, A.x + 2 * A.y, A.y);
            let Y = 0.25 * vec4<f32>(B.x, 2 * B.x + B.y, B.x + 2 * B.y, B.y);
            let dX = 0.5 * vec4<f32>(-A.x, -A.y, A.x, A.y);
            let dY = 0.5 * vec4<f32>(-B.x, -B.y, B.x, B.y);
            let height_duv_upscaled = texture_size * vec2(dot(Y, dX * height_matrix), dot(dY, X * height_matrix));

            height_duv = mix(height_duv, height_duv_upscaled, ratio);
            // height_duv = height_duv_upscaled;
        }

        height_dx = dot(height_duv, uv_dx);
        height_dy = dot(height_duv, uv_dy);
    }

    return tangent_space.scale * (height_dx * tangent_space.tangent_x + height_dy * tangent_space.tangent_y);
}
#endif

fn sample_color(tile: AtlasTile) -> vec4<f32> {
    let height = sample_attachment0(tile).x;

    return vec4<f32>(height * 0.5);
}

fn compute_slope(world_normal: vec3<f32>, surface_gradient: vec3<f32>) -> f32 {
    let normal  = normalize(world_normal - surface_gradient);
    let cos_slope = min(dot(normal, world_normal), 1.0); // avoid artifacts
    return acos(cos_slope); // slope in radians
}
