#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, NodeLookup
#import bevy_terrain::bindings config, atlas_sampler
#import bevy_terrain::functions vertex_local_position, vertex_blend, lookup_node, compute_blend, local_to_world_position
#import bevy_terrain::debug show_tiles, show_lod, show_quadtree
#import bevy_terrain::attachments height_atlas, HEIGHT_SCALE, HEIGHT_OFFSET
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::pbr_functions PbrInput, pbr_input_new, calculate_view, pbr

@group(3) @binding(0)
var gradient: texture_1d<f32>;

fn terrain_height(lookup: NodeLookup) -> f32 {
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let height = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x - 1.0;

    return config.height * height;
}

// Todo: fix this faulty implementation
fn terrain_normal(lookup: NodeLookup, local_position: vec3<f32>) -> vec3<f32> {
    let normal = normalize(local_position);
    let tangent = cross(vec3(0.0, 1.0, 0.0), normal);
    let bitangent = -cross(tangent, normal);
    let TBN = mat3x3<f32>(tangent, bitangent, normal);

    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;

    let left  = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>(-1,  0)).x - 1.0;
    let up    = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>( 0, -1)).x - 1.0;
    let right = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>( 1,  0)).x - 1.0;
    let down  = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0, vec2<i32>( 0,  1)).x - 1.0;

    let surface_normal = normalize(vec3<f32>(right - left, down - up, f32(2u << lookup.atlas_lod) / 300.0));

    return normalize(TBN * surface_normal);
}

fn terrain_color(lookup: NodeLookup) -> vec4<f32> {
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let height = 2.0 * textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x - 1.0;

    if (height < 0.0) {
        return textureSampleLevel(gradient, atlas_sampler, mix(0.0, 0.075, pow(-height, 0.25)), 0.0);
    }
    else {
        return textureSampleLevel(gradient, atlas_sampler, mix(0.09, 1.0, pow(height * 6.0, 1.75)), 0.0);
    }
}

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let local_position = vertex_local_position(input.vertex_index);
    let blend = vertex_blend(local_position);

    let lookup = lookup_node(local_position, blend.lod);
    var height = terrain_height(lookup);

    if (blend.ratio < 1.0) {
        let lookup2 = lookup_node(local_position, blend.lod + 1u);
        height      = mix(height, terrain_height(lookup2), blend.ratio);
    }

    let world_position = local_to_world_position(local_position, height);

    var output: VertexOutput;
    output.fragment_position = view.view_proj * world_position;
    output.local_position    = local_position;
    output.world_position    = world_position;
    output.debug_color       = show_tiles(input.vertex_index, world_position);

    return output;
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    let blend = compute_blend(input.world_position);

    let lookup       = lookup_node(input.local_position, blend.lod);
    var height       = terrain_height(lookup);
    var world_normal = terrain_normal(lookup, input.local_position);
    var color        = terrain_color(lookup);

    if (blend.ratio < 1.0) {
        let lookup2  = lookup_node(input.local_position, blend.lod + 1u);
        height       = mix(height,       terrain_height(lookup2),                       blend.ratio);
        world_normal = mix(world_normal, terrain_normal(lookup2, input.local_position), blend.ratio);
        color        = mix(color,        terrain_color(lookup2),                        blend.ratio);
    }

    let opacity = 0.8;

#ifdef LIGHTING
    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.base_color           = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance          = 0.0;
    pbr_input.frag_coord                    = input.fragment_position;
    pbr_input.world_position                = input.world_position;
    pbr_input.world_normal                  = world_normal;
    pbr_input.is_orthographic               = view.projection[3].w == 1.0;
    pbr_input.N                             = world_normal;
    pbr_input.V                             = calculate_view(input.world_position, pbr_input.is_orthographic);
    color = pbr(pbr_input);
#endif
#ifdef SHOW_LOD
    color = mix(color, show_lod(input.local_position, input.world_position, lookup.atlas_lod), opacity);
#endif
#ifdef SHOW_UV
    color = mix(color, vec4<f32>(lookup.atlas_coordinate, 0.0, 1.0)                          , opacity);
#endif
#ifdef SHOW_TILES
    color = mix(color, input.debug_color                                                     , opacity);
#endif
#ifdef SHOW_QUADTREE
    color = mix(color, show_quadtree(input.local_position)                                   , opacity);
#endif

    return FragmentOutput(color);
}
