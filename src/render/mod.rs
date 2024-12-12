//! This module contains the implementation of the Uniform Distance-Dependent Level of Detail (UDLOD).
//!
//! This algorithm is responsible for approximating the terrain geometry.
//! Therefore tiny mesh tiles are refined in a tile_tree-like manner in a compute shader prepass for
//! each view. Then they are drawn using a single draw indirect call and morphed together to form
//! one continuous surface.

pub(crate) mod terrain_bind_group;
pub(crate) mod terrain_material;
pub(crate) mod terrain_pass;
pub(crate) mod terrain_view_bind_group;
pub(crate) mod tiling_prepass;

pub use crate::render::{
    terrain_bind_group::TerrainData,
    terrain_material::TerrainMaterial,
    terrain_material::TerrainMaterialPlugin,
    terrain_view_bind_group::{GpuTerrainView, TerrainView},
};
