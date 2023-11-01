#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile, S2Coordinate
#import bevy_terrain::bindings config, view_config, tiles, atlas_sampler
#import bevy_terrain::functions vertex_local_position, approximate_world_position, s2_from_world_position, lookup_node, blend, nodes_per_side, s2_project_to_side, node_coordinate, quadtree_lod, inside_rect, s2_to_world_position
#import bevy_terrain::debug index_color, show_tiles, show_lod, quadtree_outlines, show_quadtree
#import bevy_terrain::attachments height_atlas, HEIGHT_SIZE, HEIGHT_SCALE, HEIGHT_OFFSET
#import bevy_pbr::mesh_view_bindings view

@group(3) @binding(0)
var cube_map: texture_2d_array<f32>;
@group(3) @binding(1)
var gradient: texture_1d<f32>;

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

    let direction = normalize(local_position);

    let s2 = s2_from_world_position(world_position);

    let scale = 2.0 * textureSampleLevel(cube_map, atlas_sampler, s2.st, s2.side, 0.0).x - 1.0;
    //let height = 40.0 * sign(scale) * pow(abs(scale), 1.5);
    let height = 40.0 * sign(scale) * pow(abs(scale), 1.5);

    world_position = world_position + vec4<f32>(direction * height, 0.0);

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
    var height: f32;

    let blend = blend(in.world_position);
    var lod = blend.lod;
    // lod = quadtree_lod(in.world_position);

    let s2 = s2_from_world_position(in.world_position);

    let cube_height = textureSampleLevel(cube_map, atlas_sampler, s2.st, s2.side, 0.0).x;

    let lookup = lookup_node(in.world_position, lod);
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let atlas_height = textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x;

    let is_outline = quadtree_outlines(in.world_position, lod);

    var color: vec4<f32>;
    // color = terrain_color(cube_height);
    color = terrain_color(atlas_height);

    // color = mix(color, show_lod(in.world_position), 0.5);
    // color = index_color(lookup.atlas_lod);
    // color = mix(color, show_quadtree(in.world_position), 1.0);

    color = mix(color, 0.1 * index_color(lookup.atlas_lod), is_outline);

    // color = vec4<f32>(lookup.atlas_coordinate, 0.0, 1.0);
    // color = vec4<f32>(height);
    // color = lod_color(side);
    // color = vec4<f32>(st.x, st.y, 0.0, 1.0);
    // color = vec4<f32>(height_coordinate.x, height_coordinate.y, 0.0, 1.0);
    // color = in.debug_color;

    return FragmentOutput(color);
}
