//! This module contains the implementation of the Uniform Distance-Dependent Level of Detail (UDLOD).
//!
//! This algorithm is responsible for approximating the terrain geometry.
//! Therefore tiny mesh tiles are refined in a tile_tree-like manner in a compute shader prepass for
//! each view. Then they are drawn using a single draw indirect call and morphed together to form
//! one continuous surface.

mod culling_bind_group;
mod terrain_bind_group;
mod terrain_material;
mod terrain_view_bind_group;
mod tiling_prepass;

pub use crate::render::{
    terrain_material::TerrainMaterialPlugin, terrain_view_bind_group::GpuTerrainView,
};

pub(crate) use crate::render::{
    culling_bind_group::{create_culling_layout, CullingBindGroup},
    terrain_bind_group::SetTerrainBindGroup,
    terrain_view_bind_group::{
        create_prepare_indirect_layout, create_refine_tiles_layout, create_terrain_view_layout,
        DrawTerrainCommand, SetTerrainViewBindGroup,
    },
    tiling_prepass::{
        queue_tiling_prepass, TilingPrepassItem, TilingPrepassLabel, TilingPrepassNode,
        TilingPrepassPipelines,
    },
};
