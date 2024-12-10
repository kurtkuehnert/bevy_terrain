//! Types for configuring terrains.
//!
#[cfg(feature = "high_precision")]
use crate::big_space::{GridCell, GridTransformOwned, ReferenceFrame};

use crate::{
    math::TerrainModel,
    terrain_data::{AttachmentConfig, TileAtlas},
};
use bevy::{ecs::entity::EntityHashMap, prelude::*};

/// Resource that stores components that are associated to a terrain entity.
/// This is used to persist components in the render world.
#[derive(Deref, DerefMut, Resource)]
pub struct TerrainComponents<C>(EntityHashMap<C>);

impl<C> Default for TerrainComponents<C> {
    fn default() -> Self {
        Self(default())
    }
}

/// The configuration of a terrain.
///
/// Here you can define all fundamental parameters of the terrain.
#[derive(Clone)]
pub struct TerrainConfig {
    /// The count of level of detail layers.
    pub lod_count: u32,
    pub model: TerrainModel,
    /// The amount of tiles the can be loaded simultaneously in the tile atlas.
    pub atlas_size: u32,
    /// The path to the terrain folder inside the assets directory.
    pub path: String,
    /// The attachments of the terrain.
    pub attachments: Vec<AttachmentConfig>,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            lod_count: 1,
            model: TerrainModel::sphere(default(), 1.0),
            atlas_size: 1024,
            path: default(),
            attachments: default(),
        }
    }
}

impl TerrainConfig {
    pub fn add_attachment(mut self, attachment_config: AttachmentConfig) -> Self {
        self.attachments.push(attachment_config);
        self
    }
}

// Turn this into hooks

#[cfg(feature = "high_precision")]
pub fn setup_terrain(
    tile_atlas: TileAtlas,
    frame: &ReferenceFrame,
) -> (TileAtlas, Transform, GridCell) {
    let GridTransformOwned { transform, cell } = tile_atlas.model.grid_transform(frame);

    (tile_atlas, transform, cell)
}

#[cfg(not(feature = "high_precision"))]
pub fn setup_terrain(tile_atlas: TileAtlas) -> (TileAtlas, Transform) {
    let transform = tile_atlas.model.transform();

    (tile_atlas, transform)
}
