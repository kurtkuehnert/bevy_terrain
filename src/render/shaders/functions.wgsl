#define_import_path bevy_terrain::functions
 
#import bevy_terrain::types TerrainConfig,TerrainViewConfig,Tile,TileList
 
#import bevy_terrain::node NodeLookup, lookup_node, approximate_world_position
#import bevy_terrain::uniforms view_config, config, atlas_sampler, height_atlas, minmax_atlas
 
#import bevy_pbr::pbr_functions as pbr_functions
   


#import bevy_render::view  View


struct Blend {
    lod: u32,
    ratio: f32,
}

 

  
fn calculate_grid_position(grid_index: u32 ) -> vec2<u32>{
    // use first and last indices of the rows twice, to form degenerate triangles
    let row_index    = clamp(grid_index % view_config.vertices_per_row, 1u, view_config.vertices_per_row - 2u) - 1u;
    let column_index = grid_index / view_config.vertices_per_row;

    return vec2<u32>(column_index + (row_index & 1u), row_index >> 1u);
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

fn minmax(local_position: vec2<f32>, size: f32 ) -> vec2<f32> {
    let lod = u32(ceil(log2(size))) + 1u;

    if (lod >= config.lod_count) {
        return vec2<f32>(0.0, config.height);
    }

    let lookup = lookup_node(lod, local_position );
    let atlas_index = lookup.atlas_index;
    let minmax_coords = lookup.atlas_coords * config.minmax_scale + config.minmax_offset;

    let min_gather = textureGather(0, minmax_atlas, atlas_sampler, minmax_coords, atlas_index);
    let max_gather = textureGather(1, minmax_atlas, atlas_sampler, minmax_coords, atlas_index);

    var min_height = min(min(min_gather.x, min_gather.y), min(min_gather.z, min_gather.w));
    var max_height = max(max(max_gather.x, max_gather.y), max(max_gather.z, max_gather.w));

    return vec2(min_height, max_height) * config.height;
}
