#import bevy_terrain::types::{AtlasTile}
#import bevy_terrain::bindings::{terrain, attachments, attachment0, terrain_sampler}
#import bevy_terrain::attachments::{sample_height, sample_height_mask, compute_slope, sample_surface_gradient, sample_attachment1 as sample_albedo, sample_attachment0_gather0, attachment_uv}
#import bevy_terrain::fragment::{FragmentInput, FragmentOutput, fragment_info, fragment_output, fragment_debug}
#import bevy_terrain::functions::lookup_tile
#import bevy_pbr::pbr_types::{PbrInput, pbr_input_new}
#import bevy_pbr::pbr_functions::{calculate_view, apply_pbr_lighting}


@group(3) @binding(0)
var gradient: texture_1d<f32>;
@group(3) @binding(1)
var gradient_sampler: sampler;

const MIN_HEIGHT: f32 = -12000.0;
const MAX_HEIGHT: f32 =  9000.0;

fn sample_color(tile: AtlasTile) -> vec4<f32> {
    let height = sample_height(tile);

    var color: vec4<f32>;

    if (height < 0.0) {
        color = textureSampleLevel(gradient, gradient_sampler, mix(0.0, 0.075, pow(height / MIN_HEIGHT, 0.25)), 0.0);
    }
    else {
        color = textureSampleLevel(gradient, gradient_sampler, mix(0.09, 1.0, pow(height / MAX_HEIGHT * 2.0, 1.0)), 0.0);
    }

    let albedo = sample_albedo(tile);

    if (!all(albedo == vec4<f32>(1.0))) {
        color = sample_albedo(tile);
    }

    return color;
}

fn slope_gradient(world_normal: vec3<f32>, surface_gradient: vec3<f32>) -> vec4<f32> {
    let slope = compute_slope(world_normal, surface_gradient);
    return textureSampleLevel(gradient, gradient_sampler, slope + 0.1, 0.0);
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

    // color = vec4(vec3(0.3), 1.0);

    // color = slope_gradient(info.world_normal, surface_gradient);

    var output: FragmentOutput;
    fragment_output(&info, &output, color, surface_gradient);
    fragment_debug(&info, &output, tile, surface_gradient);
    return output;
}
