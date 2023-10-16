#define_import_path bevy_terrain::bindings

#import bevy_terrain::types TerrainViewConfig, TerrainConfig, TileList

// terrain view bindings
@group(1) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(1) @binding(1)
var quadtree: texture_2d_array<u32>;
@group(1) @binding(2)
var<storage> tiles: TileList;

// terrain bindings
@group(2) @binding(1)
var<uniform> config: TerrainConfig;
@group(2) @binding(2)
var atlas_sampler: sampler;
