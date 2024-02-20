#define_import_path bevy_terrain::bindings

#import bevy_terrain::types::{TerrainViewConfig, TerrainConfig, Quadtree, TileList, AttachmentList}

// terrain bindings
@group(1) @binding(1)
var<uniform> config: TerrainConfig;
@group(1) @binding(2)
var<uniform> attachments: AttachmentList;
@group(1) @binding(3)
var atlas_sampler: sampler;
@group(1) @binding(4)
var attachment0_atlas: texture_2d_array<f32>;
@group(1) @binding(5)
var attachment1_atlas: texture_2d_array<f32>;
@group(1) @binding(6)
var attachment2_atlas: texture_2d_array<f32>;
@group(1) @binding(7)
var attachment3_atlas: texture_2d_array<f32>;
@group(1) @binding(8)
var attachment4_atlas: texture_2d_array<f32>;
@group(1) @binding(9)
var attachment5_atlas: texture_2d_array<f32>;
@group(1) @binding(10)
var attachment6_atlas: texture_2d_array<f32>;
@group(1) @binding(11)
var attachment7_atlas: texture_2d_array<f32>;

// terrain view bindings
@group(2) @binding(0)
var<uniform> view_config: TerrainViewConfig;
@group(2) @binding(1)
var<storage> quadtree: Quadtree;
@group(2) @binding(2)
var<storage> tiles: TileList;
