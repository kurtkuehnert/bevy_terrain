#define_import_path bevy_terrain::functions

struct VertexInput {
    @builtin(instance_index) instance: u32,
    @builtin(vertex_index)   vertex_index: u32,
}

struct VertexOutput {
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec2<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}

fn vertex_output(local_position: vec2<f32>, height: f32) -> VertexOutput {
    var world_position = vec4<f32>(local_position.x, height, local_position.y, 1.0);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = vec2<f32>(local_position);
    output.world_position = world_position;
    output.debug_color = vec4<f32>(0.0);

    return output;
}

struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec2<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}

struct FragmentOutput {
    @location(0)             color: vec4<f32>
}

// The processed fragment consisting of the color and a flag whether or not to discard this fragment.
struct Fragment {
    color: vec4<f32>,
    do_discard: bool,
}

struct Blend {
    lod: u32,
    ratio: f32,
}

fn calculate_blend(world_position: vec4<f32>) -> Blend {
    let viewer_distance = distance(world_position.xyz, view.world_position.xyz);
    let log_distance = max(log2(2.0 * viewer_distance / view_config.blend_distance), 0.0);
    let ratio = (1.0 - log_distance % 1.0) / view_config.blend_range;

    return Blend(u32(log_distance), ratio);
}

fn calculate_morph(tile: Tile, world_position: vec4<f32>) -> f32 {
    let viewer_distance = distance(world_position.xyz, view.world_position.xyz);
    let morph_distance = view_config.morph_distance * f32(tile.size << 1u);

    return clamp(1.0 - (1.0 - viewer_distance / morph_distance) / view_config.morph_range, 0.0, 1.0);
}

fn calculate_grid_position(grid_index: u32) -> vec2<u32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;

    return vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);
}

fn calculate_local_position(tile: Tile, grid_position: vec2<u32>) -> vec2<f32> {
    let size = f32(tile.size) * view_config.tile_scale;

    var local_position = (vec2<f32>(tile.coords) + vec2<f32>(grid_position) / view_config.grid_size) * size;

#ifdef MESH_MORPH
    let world_position = approximate_world_position(local_position);
    let morph = calculate_morph(tile, world_position);
    let even_grid_position = vec2<f32>(grid_position & vec2<u32>(1u));
    local_position = local_position - morph * even_grid_position / view_config.grid_size * size;
#endif

    local_position.x = clamp(local_position.x, 0.0, f32(config.terrain_size));
    local_position.y = clamp(local_position.y, 0.0, f32(config.terrain_size));

    return local_position;
}

fn calculate_normal(coords: vec2<f32>, atlas_index: i32, atlas_lod: u32, ddx: vec2<f32>, ddy: vec2<f32>) -> vec3<f32> {
#ifdef SAMPLE_GRAD
    let offset = 1.0 / config.height_size;
    let left  = textureSampleGrad(height_atlas, atlas_sampler, coords + vec2<f32>(-offset,     0.0), atlas_index, ddx, ddy).x;
    let up    = textureSampleGrad(height_atlas, atlas_sampler, coords + vec2<f32>(    0.0, -offset), atlas_index, ddx, ddy).x;
    let right = textureSampleGrad(height_atlas, atlas_sampler, coords + vec2<f32>( offset,     0.0), atlas_index, ddx, ddy).x;
    let down  = textureSampleGrad(height_atlas, atlas_sampler, coords + vec2<f32>(    0.0,  offset), atlas_index, ddx, ddy).x;
#else
    let left  = textureSampleLevel(height_atlas, atlas_sampler, coords, atlas_index, 0.0, vec2<i32>(-1,  0)).x;
    let up    = textureSampleLevel(height_atlas, atlas_sampler, coords, atlas_index, 0.0, vec2<i32>( 0, -1)).x;
    let right = textureSampleLevel(height_atlas, atlas_sampler, coords, atlas_index, 0.0, vec2<i32>( 1,  0)).x;
    let down  = textureSampleLevel(height_atlas, atlas_sampler, coords, atlas_index, 0.0, vec2<i32>( 0,  1)).x;

#endif

    return normalize(vec3<f32>(right - left, f32(2u << atlas_lod) / config.height, down - up));
}

fn minmax(local_position: vec2<f32>, size: f32) -> vec2<f32> {
    let lod = u32(ceil(log2(size))) + 1u;

    if (lod >= config.lod_count) {
        return vec2<f32>(0.0, config.height);
    }

    let lookup = lookup_node(lod, local_position);
    let atlas_index = lookup.atlas_index;
    let minmax_coords = lookup.atlas_coords * config.minmax_scale + config.minmax_offset;

    let min_gather = textureGather(0, minmax_atlas, atlas_sampler, minmax_coords, atlas_index);
    let max_gather = textureGather(1, minmax_atlas, atlas_sampler, minmax_coords, atlas_index);

    var min_height = min(min(min_gather.x, min_gather.y), min(min_gather.z, min_gather.w));
    var max_height = max(max(max_gather.x, max_gather.y), max(max_gather.z, max_gather.w));

    return vec2(min_height, max_height) * config.height;
}
