#import bevy_terrain::types VertexInput, VertexOutput, FragmentInput, FragmentOutput, Tile
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

fn show_quadtree(world_position: vec4<f32>) -> vec4<f32> {


    // const NS: u32 = 4u;
    // const NT: u32 = 5u;

    // var EVEN_LIST = array<vec2<u32>, 6u>(
    // );
    // var ODD_LIST = array<vec2<u32>, 6u>(
    // );

    var SIDE_MATRIX = array<vec2<u32>, 36u>(
        vec2<u32>(PS, PT), // side 0 to side 0
        vec2<u32>(F0, PT), // side 0 to side 1
        vec2<u32>(F0, PS), // side 0 to side 2
        vec2<u32>(PT, PS), // side 0 to side 3
        vec2<u32>(PT, F0), // side 0 to side 4
        vec2<u32>(PS, F0), // side 0 to side 5

        vec2<u32>(F1, PT), // side 1 to side 0
        vec2<u32>(PS, PT), // side 1 to side 1
        vec2<u32>(PS, F1), // side 1 to side 2
        vec2<u32>(PT, F1), // side 1 to side 3
        vec2<u32>(PT, PS), // side 1 to side 4
        vec2<u32>(F1, PS), // side 1 to side 5

        vec2<u32>(PT, F0), // side 2 to side 0
        vec2<u32>(PS, F0), // side 2 to side 1
        vec2<u32>(PS, PT), // side 2 to side 2
        vec2<u32>(F0, PT), // side 2 to side 3
        vec2<u32>(F0, PS), // side 2 to side 4
        vec2<u32>(PS, F0), // side 2 to side 5

        vec2<u32>(PT, PS), // side 3 to side 0
        vec2<u32>(F1, PS), // side 3 to side 1
        vec2<u32>(F1, PT), // side 3 to side 2
        vec2<u32>(PS, PT), // side 3 to side 3
        vec2<u32>(PS, F1), // side 3 to side 4
        vec2<u32>(PT, F1), // side 3 to side 5

        vec2<u32>(F0, PS), // side 4 to side 0
        vec2<u32>(PT, PS), // side 4 to side 1
        vec2<u32>(PT, F0), // side 4 to side 2
        vec2<u32>(PS, F0), // side 4 to side 3
        vec2<u32>(PS, PT), // side 4 to side 4
        vec2<u32>(F0, PT), // side 4 to side 5

        vec2<u32>(PS, F1), // side 5 to side 0
        vec2<u32>(PT, F1), // side 5 to side 1
        vec2<u32>(PT, PS), // side 5 to side 2
        vec2<u32>(F1, PS), // side 5 to side 3
        vec2<u32>(F1, PT), // side 5 to side 4
        vec2<u32>(PS, PT), // side 5 to side 5
    );

    let view_s2_coordinate = world_position_to_s2_coordinate(vec4<f32>(view.world_position, 1.0));
    let view_st = view_s2_coordinate.st;
    let view_side = view_s2_coordinate.side;

    let s2_coordinate = world_position_to_s2_coordinate(world_position);
    let st = s2_coordinate.st;
    let side = s2_coordinate.side;

    let blend = blend(world_position);
    var lod = blend.lod;
    lod = 0u;

    var color: vec4<f32> = index_color(lod);

    let node_count = node_count(lod);

    let view_node_coordinate = view_st * node_count;
    let node_coordinate      = st * node_count;


    if (side == view_side && distance(st, view_st) < 0.005) {
        color = 0.0 * color;
    }

    let thickness = 0.01;

    let atlas_coordinate = node_coordinate % 1.0;


    let info = SIDE_MATRIX[6u * view_side + side];

    var test_st: vec2<f32>;
    if (info.x == F0) { test_st.x = 0.0; }
    if (info.x == F1) { test_st.x = 1.0; }
    if (info.x == PS) { test_st.x = view_st.x; }
    if (info.x == PT) { test_st.x = view_st.y; }

    if (info.y == F0) { test_st.y = 0.0; }
    if (info.y == F1) { test_st.y = 1.0; }
    if (info.y == PS) { test_st.y = view_st.x; }
    if (info.y == PT) { test_st.y = view_st.y; }

    let test_node_coordinate = test_st * node_count;

    let quadtree_size = i32(view_config.node_count);
    var quadtree_origin = vec2<i32>(round(test_node_coordinate - 0.5 * f32(quadtree_size)));

    quadtree_origin = clamp(quadtree_origin, vec2<i32>(0), vec2<i32>(i32(ceil(node_count)) - quadtree_size));

    let node_under_frag = vec2<i32>(node_coordinate);
    let dist = node_under_frag - quadtree_origin;

    if (dist.x < 0 || dist.y < 0 || dist.x >= quadtree_size || dist.y >= quadtree_size) {
        color = 0.3 * color;
    }




/*    if (side != view_side) {
        color = 0.3 * color;


    }
    else {
        let node_count = i32(view_config.node_count);
        let lower = vec2<i32>(round(view_node_coordinate - 0.5 * f32(view_config.node_count)));
        let node_under_frag = vec2<i32>(node_coordinate);
        let dist = node_under_frag - lower;

        if (dist.x < 0 || dist.y < 0 || dist.x >= node_count || dist.y >= node_count) {
            color = 0.3 * color;
        }
    }*/

    let grid_outer = step(vec2<f32>(0.0)            , atlas_coordinate) * step(atlas_coordinate, vec2<f32>(1.0));
    let grid_inner = step(vec2<f32>(0.0) + thickness, atlas_coordinate) * step(atlas_coordinate, vec2<f32>(1.0) - thickness);
    let outline = grid_outer.x * grid_outer.y - grid_inner.x * grid_inner.y;



//        let node_size = node_size(lod);
//        let grid_position = floor(view.world_position.xz / node_size + 0.5 - f32(view_config.node_count >> 1u)) * node_size;
//        let grid_size = node_size * f32(view_config.node_count);
//        let thickness = f32(8u << lod);
//
//        let grid_outer = step(grid_position, world_position.xz) * step(world_position.xz, grid_position + grid_size);
//        let grid_inner = step(grid_position + thickness, world_position.xz) * step(world_position.xz, grid_position + grid_size - thickness);
//        let outline = grid_outer.x * grid_outer.y - grid_inner.x * grid_inner.y;

    color = mix(color, index_color(lod) * 0.1, outline);


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
    let height = 20.0 * sign(scale) * pow(abs(scale), 1.5);

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
