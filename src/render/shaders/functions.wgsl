#define_import_path bevy_terrain::functions

struct VertexInput {
    @builtin(instance_index) instance: u32,
    @builtin(vertex_index)   index: u32,
}

struct VertexOutput {
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec2<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             color: vec4<f32>,
#ifdef VERTEX_NORMAL
    @location(3)             world_normal: vec3<f32>,
#endif
}

struct FragmentInput {
    @builtin(front_facing)   is_front: bool,
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec2<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             color: vec4<f32>,
#ifdef VERTEX_NORMAL
    @location(3)             world_normal: vec3<f32>,
#endif
}

struct FragmentOutput {
    @location(0)             color: vec4<f32>
}

struct Blend {
    ratio: f32,
    log_distance: f32,
}

fn calculate_blend(world_position: vec3<f32>, blend_range: f32) -> Blend {
    let viewer_distance = distance(world_position, view.world_position.xyz);
    let log_distance = log2(2.0 * viewer_distance / view_config.view_distance);
    let ratio = (1.0 - log_distance % 1.0) / blend_range;

    return Blend(ratio, log_distance);
}

fn calculate_morph(local_position: vec2<f32>, tile: Tile) -> f32 {
    let world_position = approximate_world_position(local_position);
    let viewer_distance = distance(world_position, view.world_position.xyz);
    let morph_distance = view_config.refinement_distance * f32(tile.size << 1u);

    return clamp(1.0 - (1.0 - viewer_distance / morph_distance) / view_config.morph_blend, 0.0, 1.0);
}

fn map_position(tile: Tile, grid_position: vec2<u32>, count: u32, true_count: u32) -> vec2<f32> {
     var position = grid_position;

     let d = true_count - count;
     let h = (count) >> 1u;
     let a = position.x > h;
     let b = position.y > h;

     if (a) {
         position.x = max(position.x, h + d) - d;
     }

     if (b) {
         position.y = max(position.y, h + d) - d;
     }

     return (vec2<f32>(tile.coords) + vec2<f32>(position) / f32(count)) * f32(tile.size) * view_config.tile_scale;
 }

fn calculate_position(vertex_index: u32, tile: Tile, vertices_per_row: u32, true_count: u32) -> vec2<f32> {
    // use first and last index twice, to form degenerate triangles
    // Todo: documentation
    let row_index    = clamp(vertex_index % vertices_per_row, 1u, vertices_per_row - 2u) - 1u;
    let column_index = vertex_index / vertices_per_row;
    var grid_position = vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);

    let size = f32(tile.size) * view_config.tile_scale;

#ifdef ADAPTIVE
    var count        = (tile.counts        >> 24u) & 0x003Fu;
    var parent_count = (tile.parent_counts >> 24u) & 0x003Fu;

    // override edge counts, so that they behave like their neighbours
    if (grid_position.x == 0u) {
        count        = (tile.counts        >>  0u) & 0x003Fu;
        parent_count = (tile.parent_counts >>  0u) & 0x003Fu;
    }
    if (grid_position.y == 0u) {
        count        = (tile.counts        >>  6u) & 0x003Fu;
        parent_count = (tile.parent_counts >>  6u) & 0x003Fu;
    }
    if (grid_position.x == true_count) {
        count        = (tile.counts        >> 12u) & 0x003Fu;
        parent_count = (tile.parent_counts >> 12u) & 0x003Fu;
    }
    if (grid_position.y == true_count) {
        count        = (tile.counts        >> 18u) & 0x003Fu;
        parent_count = (tile.parent_counts >> 18u) & 0x003Fu;
    }

    #ifdef MESH_MORPH
        // smoothly transition between the positions of the tiles and that of their parents
        var local_position        = map_position(tile, grid_position, count,        true_count);
        let parent_local_position = map_position(tile, grid_position, parent_count, true_count);

        let morph = calculate_morph(local_position, tile);

        local_position = mix(local_position, parent_local_position, morph);
    #else
        var local_position = (vec2<f32>(tile.coords) + vec2<f32>(grid_position) / f32(true_count)) * size;
    #endif
#else
    var local_position = (vec2<f32>(tile.coords) + vec2<f32>(grid_position) / f32(true_count)) * size;

    #ifdef MESH_MORPH
        let morph = calculate_morph(local_position, tile);
        let even_grid_position = vec2<f32>(grid_position & vec2<u32>(1u));
        local_position = local_position - morph * even_grid_position / f32(true_count) * size;
    #endif
#endif

    local_position.x = clamp(local_position.x, 0.0, f32(config.terrain_size));
    local_position.y = clamp(local_position.y, 0.0, f32(config.terrain_size));

    return local_position;
}

fn calculate_normal(uv: vec2<f32>, atlas_index: i32, lod: u32) -> vec3<f32> {
    let left  = textureSampleLevel(height_atlas, terrain_sampler, uv, atlas_index, 0.0, vec2<i32>(-1,  0)).x;
    let up    = textureSampleLevel(height_atlas, terrain_sampler, uv, atlas_index, 0.0, vec2<i32>( 0, -1)).x;
    let right = textureSampleLevel(height_atlas, terrain_sampler, uv, atlas_index, 0.0, vec2<i32>( 1,  0)).x;
    let down  = textureSampleLevel(height_atlas, terrain_sampler, uv, atlas_index, 0.0, vec2<i32>( 0,  1)).x;

    return normalize(vec3<f32>(right - left, f32(2u << lod) / config.height, down - up));
}

fn vertex_output(local_position: vec2<f32>, height: f32) -> VertexOutput {
    let world_position = vec4<f32>(local_position.x, height, local_position.y, 1.0);

    var output: VertexOutput;
    output.frag_coord = view.view_proj * world_position;
    output.local_position = vec2<f32>(local_position);
    output.world_position = world_position;
    output.color = vec4<f32>(0.0);

    return output;
}
