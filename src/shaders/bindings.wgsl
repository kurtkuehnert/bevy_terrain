#define_import_path bevy_terrain::bindings

#import bevy_terrain::types::{TerrainView, Terrain, TileTreeEntry, TileCoordinate, AttachmentConfig, TerrainModelApproximation, CullingData, IndirectBuffer, PrepassState}
#import bevy_pbr::mesh_types::Mesh

// terrain bindings
@group(1) @binding(0)
var<storage> mesh: array<Mesh>; // Todo: replace with custom Mesh uniform / include in terrain
@group(1) @binding(1)
var<uniform> terrain: Terrain;
@group(1) @binding(2)
var<uniform> attachments: array<AttachmentConfig, 8u>;
@group(1) @binding(3)
var terrain_sampler: sampler;
@group(1) @binding(4)
var attachment0: texture_2d_array<f32>;
@group(1) @binding(5)
var attachment1: texture_2d_array<f32>;
@group(1) @binding(6)
var attachment2: texture_2d_array<f32>;
@group(1) @binding(7)
var attachment3: texture_2d_array<f32>;
@group(1) @binding(8)
var attachment4: texture_2d_array<f32>;
@group(1) @binding(9)
var attachment5: texture_2d_array<f32>;
@group(1) @binding(10)
var attachment6: texture_2d_array<f32>;
@group(1) @binding(11)
var attachment7: texture_2d_array<f32>;

// terrain view bindings
#ifndef PREPASS
@group(2) @binding(0)
var<storage> terrain_view: TerrainView;
@group(2) @binding(1)
var<storage> approximate_height: f32;
@group(2) @binding(2)
var<storage> tile_tree: array<TileTreeEntry>;
@group(2) @binding(3)
var<storage> geometry_tiles: array<TileCoordinate>;
#endif PREPASS

// refine tiles bindings
#ifdef PREPASS
@group(0) @binding(0)
var<storage> terrain_view: TerrainView;
@group(0) @binding(1)
var<storage, read_write> approximate_height_write: f32; // Todo: consider using shaderdefs instead of rename for read/read_write
@group(0) @binding(2)
var<storage> tile_tree: array<TileTreeEntry>;
@group(0) @binding(3)
var<storage, read_write> final_tiles: array<TileCoordinate>;
@group(0) @binding(4)
var<storage, read_write> temporary_tiles: array<TileCoordinate>;
@group(0) @binding(5)
var<storage, read_write> state: PrepassState;
@group(0) @binding(6)
var<storage> culling_view: CullingData;

@group(2) @binding(0)
var<storage, read_write> indirect_buffer: IndirectBuffer;
#endif
