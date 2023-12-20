#define_import_path bevy_terrain::bindings

#import bevy_terrain::types::{TerrainViewConfig, TerrainConfig, Quadtree, TileList}

// terrain bindings
@group(1) @binding(1)
var<uniform> config: TerrainConfig;

// terrain view bindings
@group(2) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(2) @binding(1)
var<storage> quadtree: Quadtree;
@group(2) @binding(2)
var<storage> tiles: TileList;
