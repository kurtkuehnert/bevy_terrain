#define_import_path bevy_terrain::vertex
#import bevy_terrain::node lookup_node, approximate_world_position, NodeLookup

#import bevy_terrain::functions  calculate_grid_position, Blend , minmax
 
 
#import bevy_terrain::types TerrainConfig,TerrainViewConfig,Tile,TileList
 
 #import bevy_pbr::mesh_view_bindings view
 

//why do i get a bind conflict ? 
 
// terrain view bindings
//@group(1) @binding(0)
//var<uniform> view_config: TerrainViewConfig;
@group(1) @binding(1)
var quadtree: texture_2d_array<u32>;
@group(1) @binding(2)
var<storage> tiles: TileList;

// terrain bindings
//@group(2) @binding(0)
//var<uniform> config: TerrainConfig;
@group(2) @binding(1)
var atlas_sampler: sampler;
@group(2) @binding(2)
var height_atlas: texture_2d_array<f32>;
@group(2) @binding(3)
var minmax_atlas: texture_2d_array<f32>;
  

  
 
struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index)   vertex_index: u32,
     
   
    
    
}

struct VertexOutput {
    @builtin(position)       frag_coord: vec4<f32>,
    @location(0)             local_position: vec2<f32>,
    @location(1)             world_position: vec4<f32>,
    @location(2)             debug_color: vec4<f32>,
}


 

// The default vertex entry point, which blends the height at the fringe between two lods.
@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    
    let vertices_per_tile = u32(12); //view_config.vertices_per_tile
    
    let tile_index = vertex.instance_index / vertices_per_tile;
    let grid_index = vertex.instance_index % vertices_per_tile;

    let tile = tiles.data[tile_index];
    let grid_position = calculate_grid_position(grid_index );

    let local_position = calculate_local_position(tile, grid_position );
    let world_position = approximate_world_position(local_position );

    let blend = calculate_blend(world_position );

    let lookup = lookup_node(blend.lod, local_position);
    var height = vertex_height(lookup);

    if (blend.ratio < 1.0) {
        let lookup2 = lookup_node(blend.lod + 1u, local_position);
        let height2 = vertex_height(lookup2);
        height      = mix(height2, height, blend.ratio);
    }

    var output = vertex_output(local_position, height);

#ifdef SHOW_TILES
    output.debug_color = show_tiles(tile, output.world_position);
#endif

#ifdef SHOW_MINMAX_ERROR
    output.debug_color = show_minmax_error(tile, height);
#endif

#ifdef TEST2
    output.debug_color = mix(output.debug_color, vec4<f32>(f32(tile_index) / 1000.0, 0.0, 0.0, 1.0), 0.4);
#endif

    return output;
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





// The function that evaluates the height of the vertex.
// This will happen once or twice (lod fringe).
// fn vertex_height(lookup: AtlasLookup) -> f32;

fn vertex_height(lookup: NodeLookup) -> f32 {
    let height_coords = lookup.atlas_coords ; //* config.height_scale + config.height_offset;
    let height = textureSampleLevel(height_atlas, atlas_sampler, height_coords, lookup.atlas_index, 0.0).x;

    return height ; // * config.height;
}



fn calculate_local_position(tile: Tile, grid_position: vec2<u32> ) -> vec2<f32> {
    
    let tile_scale = 1.0; //view_config.tile_scale;
    let grid_size = 10.0; //view_config.grid_size;;
    
    let size = f32(tile.size) * tile_scale;

    var local_position = (vec2<f32>(tile.coords) + vec2<f32>(grid_position) / grid_size) * size;

#ifdef MESH_MORPH
    let world_position = approximate_world_position(local_position);
    let morph = calculate_morph(tile, world_position);
    let even_grid_position = vec2<f32>(grid_position & vec2<u32>(1u));
    local_position = local_position - morph * even_grid_position / grid_size * size;
#endif

    let terrain_size = 1024.0; //config.terrain_size;

    local_position.x = clamp(local_position.x, 0.0, f32(terrain_size));
    local_position.y = clamp(local_position.y, 0.0, f32(terrain_size));

    return local_position;
}




fn calculate_morph(tile: Tile, world_position: vec4<f32> ) -> f32 {
    
    let config_morph_distance = 30.0; //view_config.morph_distance;
    let config_morph_range = 20.0; //view_config.morph_range;
    
    let viewer_distance = distance(world_position.xyz, view.world_position.xyz);
    let morph_distance = config_morph_distance * f32(tile.size << 1u);

    return clamp(1.0 - (1.0 - viewer_distance / morph_distance) / config_morph_range, 0.0, 1.0);
}


fn calculate_blend(world_position: vec4<f32> ) -> Blend {
    let config_blend_distance = 30.0; //view_config.blend_distance
    let config_blend_range = 30.0; 
    
    let viewer_distance = distance(world_position.xyz, view.world_position.xyz);
    let log_distance = max(log2(2.0 * viewer_distance / config_blend_distance), 0.0);
    let ratio = (1.0 - log_distance % 1.0) / config_blend_range;

    return Blend(u32(log_distance), ratio);
}

fn show_minmax_error(tile: Tile, height: f32) -> vec4<f32> {
    
    let config_tile_scale = 1.0; //view_config.tile_scale 
    
    let size = f32(tile.size) * config_tile_scale;
    let local_position = (vec2<f32>(tile.coords) + 0.5) * size;
    let lod = u32(ceil(log2(size))) + 1u;
    let minmax = minmax(local_position, size );

    var color = vec4<f32>(0.0,
                          clamp((minmax.y - height) / size / 2.0, 0.0, 1.0),
                          clamp((height - minmax.x) / size / 2.0, 0.0, 1.0),
                          0.5);

    let tolerance = 0.00001;
    
    let lod_count = u32(3); //config.lod_count; 

    if (height < minmax.x - tolerance || height > minmax.y + tolerance  ){ // }|| lod >=  lod_count) {
        color = vec4<f32>(1.0, 0.0, 0.0, 0.5);
    }

    return color;
}




fn show_tiles(tile: Tile, world_position: vec4<f32>) -> vec4<f32> {
    var color: vec4<f32>;

    if ((tile.coords.x + tile.coords.y) % 2u == 0u) {
        color = vec4<f32>(0.5, 0.5, 0.5, 1.0);
    }
    else {
        color = vec4<f32>(0.1, 0.1, 0.1, 1.0);
    }

    let lod = u32(ceil(log2(f32(tile.size))));
    color = mix(color, lod_color(lod), 0.5);

#ifdef MESH_MORPH
    let morph = calculate_morph(tile, world_position );
    color = color + vec4<f32>(1.0, 1.0, 1.0, 1.0) * morph;
#endif

    return vec4<f32>(color.xyz, 0.5);
}

fn lod_color(lod: u32) -> vec4<f32> {
    if (lod % 6u == 0u) {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
    if (lod % 6u == 1u) {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 2u) {
        return vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 3u) {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
    if (lod % 6u == 4u) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    if (lod % 6u == 5u) {
        return vec4<f32>(0.0, 1.0, 1.0, 1.0);
    }

    return vec4<f32>(0.0);
}
