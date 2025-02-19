#define_import_path bevy_terrain::bindings

#import bevy_terrain::types::{TerrainView, Terrain, TileTreeEntry, TileCoordinate, AttachmentConfig, TerrainModelApproximation, CullingData, IndirectBuffer, PrepassState}
#import bevy_render::view::View;

struct Attachments {
    {0}: AttachmentConfig,
    {1}: AttachmentConfig,
    {2}: AttachmentConfig,
    {3}: AttachmentConfig,
    {4}: AttachmentConfig,
    {5}: AttachmentConfig,
    {6}: AttachmentConfig,
    {7}: AttachmentConfig,
}

// refine tiles bindings
#ifdef PREPASS
@group(0) @binding(0) var<storage> terrain_view: TerrainView;
@group(0) @binding(1) var<storage, read_write> approximate_height: f32;
@group(0) @binding(2) var<storage> tile_tree: array<TileTreeEntry>;
@group(0) @binding(3) var<storage, read_write> final_tiles: array<TileCoordinate>;
@group(0) @binding(4) var<storage, read_write> temporary_tiles: array<TileCoordinate>;
@group(0) @binding(5) var<storage, read_write> state: PrepassState;
@group(0) @binding(6) var<storage> culling_view: CullingData;
@group(2) @binding(0) var<storage, read_write> indirect_buffer: IndirectBuffer;
#endif

// terrain view bindings
#ifndef PREPASS
@group(0) @binding(0) var<uniform> view: View;
@group(2) @binding(0) var<storage> terrain_view: TerrainView;
@group(2) @binding(1) var<storage> approximate_height: f32;
@group(2) @binding(2) var<storage> tile_tree: array<TileTreeEntry>;
@group(2) @binding(3) var<storage> geometry_tiles: array<TileCoordinate>;
#endif PREPASS

// terrain bindings
@group(1) @binding(0)  var<storage> terrain: Terrain;
@group(1) @binding(1)  var<uniform> attachments: Attachments;
@group(1) @binding(2)  var terrain_sampler: sampler;
@group(1) @binding(3)  var {0}_attachment: texture_2d_array<f32>;
@group(1) @binding(4)  var {1}_attachment: texture_2d_array<f32>;
@group(1) @binding(5)  var {2}_attachment: texture_2d_array<f32>;
@group(1) @binding(6)  var {3}_attachment: texture_2d_array<f32>;
@group(1) @binding(7)  var {4}_attachment: texture_2d_array<f32>;
@group(1) @binding(8)  var {5}_attachment: texture_2d_array<f32>;
@group(1) @binding(9)  var {6}_attachment: texture_2d_array<f32>;
@group(1) @binding(10) var {7}_attachment: texture_2d_array<f32>;
