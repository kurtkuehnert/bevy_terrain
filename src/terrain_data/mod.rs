//! This module contains the two fundamental data structures of the terrain:
//! the [`TileTree`] and the [`TileAtlas`].
//!
//! # Explanation
//! Each terrain possesses one [`TileAtlas`], which can be configured
//! to store any [`AtlasAttachment`](tile_atlas::AtlasAttachment) required (eg. height, density, albedo, splat, edc.)
//! These attachments can vary in resolution and texture format.
//!
//! To decide which tiles should be currently loaded you can create multiple
//! [`TileTree`] views that correspond to one tile atlas.
//! These tile_trees request and release tiles from the tile atlas based on their quality
//! setting (`load_distance`).
//! Additionally they are then used to access the best loaded data at any position.
//!
//! Both the tile atlas and the tile_trees also have a corresponding GPU representation,
//! which can be used to access the terrain data in shaders.

pub(crate) mod attachment;
pub(crate) mod gpu_tile_atlas;
pub(crate) mod tile_atlas;
pub(crate) mod tile_tree;

pub use crate::terrain_data::{
    attachment::{AttachmentConfig, AttachmentFormat, AttachmentLabel},
    gpu_tile_atlas::GpuTileAtlas,
    tile_atlas::TileAtlas,
    tile_tree::TileTree,
};

pub const INVALID_ATLAS_INDEX: u32 = u32::MAX;
pub const INVALID_LOD: u32 = u32::MAX;
