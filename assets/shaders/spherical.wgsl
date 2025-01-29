#import bevy_terrain::types::{AtlasTile}
#import bevy_terrain::bindings::{terrain, terrain_view, attachments, height_attachment, albedo_atlas, albedo_attachment, terrain_sampler}
#import bevy_terrain::attachments::{sample_height, sample_height_mask, compute_slope, sample_surface_gradient}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_info, fragment_output, fragment_debug}
#import bevy_terrain::functions::{lookup_tile, inverse_mix, high_precision}
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}

struct GradientInfo {
    mode: u32,
}

@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient_sampler: sampler;
@group(3) @binding(2)
var<uniform> gradient_info: GradientInfo;

fn sample_albedo(tile: AtlasTile) -> vec4<f32> {
    let attachment = attachments.albedo;
    let uv         = tile.coordinate.uv * attachment.scale + attachment.offset;

#ifdef FRAGMENT
#ifdef SAMPLE_GRAD
    return textureSampleGrad(albedo_attachment, terrain_sampler, uv, tile.index, tile.coordinate.uv_dx, tile.coordinate.uv_dy);
#else
    return textureSampleLevel(albedo_attachment, terrain_sampler, uv, tile.index, 0.0);
#endif
#else
    return textureSampleLevel(albedo_attachment, terrain_sampler, uv, tile.index, 0.0);
#endif
}

fn color_earth(tile: AtlasTile) -> vec4<f32> {
   let height = sample_height(tile) / terrain.height_scale;

    if (height < 0.0) {
        return textureSampleLevel(gradient, gradient_sampler, mix(0.0, 0.075, pow(height / terrain.min_height, 0.25)), 0.0);
    } else {
        return textureSampleLevel(gradient, gradient_sampler, mix(0.09, 0.6, pow(height / terrain.max_height * 1.4, 1.0)), 0.0);
    }
}

fn color_dataset(tile: AtlasTile) -> vec4<f32> {
    let height = sample_height(tile) / terrain.height_scale;

    return textureSampleLevel(gradient, gradient_sampler, inverse_mix(terrain.min_height, terrain.max_height, height), 0.0);
}

fn sample_color(tile: AtlasTile) -> vec4<f32> {
    let height = sample_height(tile) / terrain.height_scale;

    var color: vec4<f32>;
    switch (gradient_info.mode) {
        case 0u: { color = color_dataset(tile); }
        case 1u: { color = color_earth(tile);   }
        case 2u: { color = sample_albedo(tile); }
        case default: {}
    }

    return color;
}

fn slope_gradient(world_normal: vec3<f32>, surface_gradient: vec3<f32>) -> vec4<f32> {
    let slope = compute_slope(world_normal, surface_gradient);
    return textureSampleLevel(gradient, gradient_sampler, 5 * slope + 0.1, 0.0);
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    var info = fragment_info(input);

    let tile             = lookup_tile(info.coordinate, info.blend, 0u);
    let mask             = sample_height_mask(tile);
    var color            = sample_color(tile);
    var surface_gradient = sample_surface_gradient(tile, info.tangent_space);

    if mask { discard; }

    if (info.blend.ratio > 0.0) {
        let tile2        = lookup_tile(info.coordinate, info.blend, 1u);
        color            = mix(color,            sample_color(tile2),                                info.blend.ratio);
        surface_gradient = mix(surface_gradient, sample_surface_gradient(tile2, info.tangent_space), info.blend.ratio);
    }

//    if (high_precision(info.world_coordinate.view_distance)) {
//        color = vec4<f32>(0.5, 0.5, 0.5, 1.0);
//    }

//    color = vec4(vec3(0.3), 1.0);

//    color = slope_gradient(info.world_coordinate.normal, surface_gradient);

    var output: FragmentOutput;
    fragment_output(&info, &output, color, surface_gradient);
    fragment_debug(&info, &output, tile, surface_gradient);
    return output;
}
