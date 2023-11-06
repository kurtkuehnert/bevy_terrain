#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile, S2Coordinate
#import bevy_terrain::bindings config, view_config, tiles, atlas_sampler
#import bevy_terrain::functions vertex_local_position, approximate_world_position, s2_from_world_position, lookup_node, blend, nodes_per_side, s2_project_to_side, node_coordinate, quadtree_lod, inside_rect, s2_to_world_position, calculate_normal
#import bevy_terrain::debug index_color, show_tiles, show_lod, quadtree_outlines, show_quadtree
#import bevy_terrain::attachments height_atlas, HEIGHT_SIZE, HEIGHT_SCALE, HEIGHT_OFFSET
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::pbr_functions PbrInput, pbr_input_new, calculate_view, pbr

@group(3) @binding(0)
var cube_map: texture_2d_array<f32>;
@group(3) @binding(1)
var gradient: texture_1d<f32>;

fn terrain_world_position(height: f32, local_position: vec3<f32>) -> vec4<f32> {
    let scale = 2.0 * height - 1.0;

    let height = config.height * scale;

    let direction = normalize(local_position);
    let local_position = local_position + vec3<f32>(direction * height);

    return vec4<f32>(local_position, 1.0);
}

fn terrain_color(height: f32) -> vec4<f32> {
    let scale = 2.0 * height - 1.0;

    let sample_ocean = textureSample(gradient, atlas_sampler, mix(0.0, 0.075, pow(-scale, 0.25)));
    let sample_land = textureSample(gradient, atlas_sampler, mix(0.09, 1.0, pow(scale * 6.0, 1.75)));

    if (scale < 0.0) {
        return sample_ocean;
    }
    else {
        return sample_land;
    }
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];

    let local_position = vertex_local_position(tile, grid_index);
    var world_position = approximate_world_position(local_position);

#ifdef TEST1
    // sample cube map
    let s2 = s2_from_world_position(world_position);
    let cube_height = textureSampleLevel(cube_map, atlas_sampler, s2.st, s2.side, 0.0).x;
    world_position = terrain_world_position(cube_height, local_position);
#else
    // sample chunked clipmap
    var lod = blend(world_position).lod;
    let lookup = lookup_node(world_position, lod);
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let atlas_height = textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x;
    world_position = terrain_world_position(atlas_height, local_position);
#endif

    var color: vec4<f32>;
    color = show_tiles(tile, world_position);
    color = mix(color, index_color(tile.side), 0.5);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = local_position;
    output.world_position = world_position;
    output.debug_color = color;

    return output;
}

@fragment
fn fragment(in: FragmentInput) -> FragmentOutput {
    // sample chunked clipmap
    let lod = blend(in.world_position).lod;
    let lookup = lookup_node(in.world_position, lod);
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let atlas_height = textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x;
    let normal = calculate_normal(in.world_position, height_coordinate, lookup.atlas_index, lookup.atlas_lod);

    let is_outline = quadtree_outlines(in.world_position, lod);

    var color: vec4<f32>;
    let opacity = 0.8;

#ifdef TEST1
    // sample cube map
    let s2 = s2_from_world_position(in.world_position);
    let cube_height = textureSampleLevel(cube_map, atlas_sampler, s2.st, s2.side, 0.0).x;
    color = terrain_color(cube_height);
#else
    color = terrain_color(atlas_height);
#endif

#ifdef SHOW_LOD
    color = mix(color, show_lod(in.world_position, lookup.atlas_lod), opacity);
#endif
#ifdef SHOW_UV
    color = mix(color, vec4<f32>(lookup.atlas_coordinate, 0.0, 1.0), opacity);
#endif
#ifdef SHOW_TILES
    color = mix(color, in.debug_color, opacity);
#endif
#ifdef SHOW_QUADTREE
    color = mix(color, show_quadtree(in.world_position), opacity);
#endif
#ifdef LIGHTING
    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance = 0.0;
    pbr_input.frag_coord = in.frag_coord;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = normal;
    pbr_input.is_orthographic = view.projection[3].w == 1.0;
    pbr_input.N = normal;
    pbr_input.V = calculate_view(in.world_position, pbr_input.is_orthographic);
    color = pbr(pbr_input);
#endif

    return FragmentOutput(color);
}
