//! This module contains the implementation of the Uniform Distance-Dependent Level of Detail (UDLOD).
//!
//! This algorithm is responsible for approximating the terrain geometry.
//! Therefore tiny mesh tiles are refined in a quadtree-like manner in a compute shader prepass for
//! each view. Then they are drawn using a single draw indirect call and morphed together to form
//! one continuous surface.

pub mod culling_bind_group;
pub mod terrain_bind_group;
pub mod terrain_material;
pub mod terrain_view_bind_group;
pub mod tiling_prepass;
