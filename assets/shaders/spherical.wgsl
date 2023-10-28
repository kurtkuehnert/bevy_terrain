#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile, S2Coordinate
#import bevy_terrain::bindings config, view_config, tiles, atlas_sampler
#import bevy_terrain::functions vertex_local_position, approximate_world_position, world_position_to_s2_coordinate, lookup_node, blend, node_count
#import bevy_terrain::debug index_color, show_tiles, show_lod
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

const F0: u32 = 0u;
const F1: u32 = 1u;
const PS: u32 = 2u;
const PT: u32 = 3u;
// const NS: u32 = 4u;
// const NT: u32 = 5u;

fn inside_rect(position: vec2<f32>, origin: vec2<f32>, size: f32) -> f32 {
    let inside = step(origin, position) * step(position, origin + size);

    return inside.x * inside.y;
}

fn inside_quadtree(lod: u32, view_s2: S2Coordinate, frag_s2: S2Coordinate) -> f32 {
    var EVEN_LIST = array<vec2<u32>, 6u>(
        vec2<u32>(PS, PT),
        vec2<u32>(F0, PT),
        vec2<u32>(F0, PS),
        vec2<u32>(PT, PS),
        vec2<u32>(PT, F0),
        vec2<u32>(PS, F0),
    );
    var ODD_LIST = array<vec2<u32>, 6u>(
        vec2<u32>(PS, PT),
        vec2<u32>(PS, F1),
        vec2<u32>(PT, F1),
        vec2<u32>(PT, PS),
        vec2<u32>(F1, PS),
        vec2<u32>(F1, PT),
    );

    let index = (6u + frag_s2.side - view_s2.side) % 6u;

    var info: vec2<u32>;
    var origin_st: vec2<f32>;

    if (view_s2.side % 2u == 0u) { info = EVEN_LIST[index]; }
    else                         { info =  ODD_LIST[index]; }

    if (info.x == F0)      { origin_st.x = 0.0; }
    else if (info.x == F1) { origin_st.x = 1.0; }
    else if (info.x == PS) { origin_st.x = view_s2.st.x; }
    else if (info.x == PT) { origin_st.x = view_s2.st.y; }

    if (info.y == F0)      { origin_st.y = 0.0; }
    else if (info.y == F1) { origin_st.y = 1.0; }
    else if (info.y == PS) { origin_st.y = view_s2.st.x; }
    else if (info.y == PT) { origin_st.y = view_s2.st.y; }

    let node_count = node_count(lod);
    let frag_node_coordinate = frag_s2.st * node_count;
    let origin_node_coordinate = origin_st * node_count;

    let quadtree_size = f32(view_config.node_count);
    let max_size = ceil(node_count) - quadtree_size;
    let quadtree_origin = clamp(round(origin_node_coordinate - 0.5 * quadtree_size), vec2<f32>(0.0), vec2<f32>(max_size));

    let dist = floor(frag_node_coordinate) - floor(quadtree_origin);

    return inside_rect(dist, vec2<f32>(0.0), quadtree_size - 1.0);
}

fn quadtree_outlines(lod: u32, frag_s2: S2Coordinate) -> f32 {
    let node_coordinate = frag_s2.st * node_count(lod);
    let atlas_coordinate = node_coordinate % 1.0;

    let thickness = 0.01;
    let outer = inside_rect(atlas_coordinate, vec2<f32>(0.0)            , 1.0);
    let inner = inside_rect(atlas_coordinate, vec2<f32>(0.0) + thickness, 1.0 - thickness);

    return outer - inner;
}

fn quadtree_lod(view_s2: S2Coordinate, frag_s2: S2Coordinate) -> u32 {
    var lod = 0u;

    loop {
        let inside_quadtree = inside_quadtree(lod, view_s2, frag_s2);

        if (inside_quadtree == 1.0 || lod == config.lod_count - 1u) { break; }

        lod = lod + 1u;
    }

    return lod;
}

fn show_quadtree(world_position: vec4<f32>) -> vec4<f32> {
    let view_s2 = world_position_to_s2_coordinate(vec4<f32>(view.world_position, 1.0));
    let frag_s2 = world_position_to_s2_coordinate(world_position);

    let blend_lod = blend(world_position).lod;
    let quadtree_lod = quadtree_lod(view_s2, frag_s2);

    var lod = max(blend_lod, quadtree_lod);
    lod = quadtree_lod;

    let is_outline = quadtree_outlines(lod, frag_s2);

    var color: vec4<f32>;

    color = mix(index_color(lod), vec4<f32>(0.0), is_outline);

    if (frag_s2.side == view_s2.side && distance(frag_s2.st, view_s2.st) < 0.005) {
        color = vec4<f32>(0.0);
    }

    return color;
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    let tile_index = in.vertex_index / view_config.vertices_per_tile;
    let grid_index = in.vertex_index % view_config.vertices_per_tile;

    let tile = tiles.data[tile_index];

    let local_position = vertex_local_position(tile, grid_index);
    var world_position = approximate_world_position(local_position);

    let direction = normalize(local_position);

    let s2_coordinate = world_position_to_s2_coordinate(world_position);
    let st = s2_coordinate.st;
    let side = s2_coordinate.side;

    let scale = 2.0 * textureSampleLevel(cube_map, atlas_sampler, st, side, 0.0).x - 1.0;
    //let height = 40.0 * sign(scale) * pow(abs(scale), 1.5);
    let height = 0.0 * sign(scale) * pow(abs(scale), 1.5);

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

    let s2_coordinate = world_position_to_s2_coordinate(in.world_position);
    let st = s2_coordinate.st;
    let side = s2_coordinate.side;
    let cube_height = textureSampleLevel(cube_map, atlas_sampler, st, side, 0.0).x;

    let blend = blend(in.world_position);
    let lookup = lookup_node(blend.lod, in.world_position);
    let height_coordinate = lookup.atlas_coordinate * HEIGHT_SCALE + HEIGHT_OFFSET;
    let atlas_height = textureSampleLevel(height_atlas, atlas_sampler, height_coordinate, lookup.atlas_index, 0.0).x;

    var color: vec4<f32>;
    color = terrain_color(cube_height);
    // color = terrain_color(atlas_height);

    // color = show_lod(in.world_position);
    color = show_quadtree(in.world_position);

    // color = vec4<f32>(lookup.atlas_coordinate, 0.0, 1.0);
    // color = vec4<f32>(height);
    // color = lod_color(side);
    // color = vec4<f32>(st.x, st.y, 0.0, 1.0);
    // color = vec4<f32>(height_coordinate.x, height_coordinate.y, 0.0, 1.0);
    // color = in.debug_color;

    return FragmentOutput(color);
}
