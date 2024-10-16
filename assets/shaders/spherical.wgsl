#import bevy_terrain::types::{AtlasTile}
#import bevy_terrain::bindings::terrain
#import bevy_terrain::attachments::{sample_height, sample_normal, sample_attachment1 as sample_albedo, sample_attachment0_gather0, attachment_uv}
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

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    var info = fragment_info(input);

    let tile   = lookup_tile(info.coordinate, info.blend, 0u);
    var color  = sample_color(tile);
    var normal = sample_normal(tile, info.world_normal);


    let raw_height = sample_attachment0_gather0(tile);
    let mask = bitcast<vec4<u32>>(raw_height) & vec4<u32>(1);

    if (any(mask == vec4<u32>(0))) { discard; }

    // color = vec4<f32>(mask) * 0.3 + 0.3;

    if (info.blend.ratio > 0.0) {
        let tile2 = lookup_tile(info.coordinate, info.blend, 1u);

        color     = mix(color,  sample_color(tile2),                     info.blend.ratio);
        normal    = mix(normal, sample_normal(tile2, info.world_normal), info.blend.ratio);
    }

    // color = vec4<f32>(0.3);

    var output: FragmentOutput;
    fragment_output(&info, &output, color, normal);
    fragment_debug(&info, &output, tile, normal);
    return output;
}
