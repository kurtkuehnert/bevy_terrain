//! Types for configuring terrains.
//!
#[cfg(feature = "high_precision")]
use crate::big_space::{GridCell, GridTransformOwned, ReferenceFrame};

use crate::prelude::TileTree;
use crate::{
    math::TerrainModel,
    terrain_data::{AttachmentConfig, TileAtlas},
};
use bevy::utils::HashMap;
use bevy::{ecs::entity::EntityHashMap, prelude::*, render::view::NoFrustumCulling};

/// Resource that stores components that are associated to a terrain entity.
/// This is used to persist components in the render world.
#[derive(Deref, DerefMut, Resource)]
pub struct TerrainComponents<C>(HashMap<AssetId<TileAtlas>, C>);

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
            model: TerrainModel::sphere(default(), 1.0, 0.0, 1.0, Entity::PLACEHOLDER),
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

/// The components of a terrain.
///
/// Does not include loader(s) and a material.
#[derive(Bundle, Default)]
pub struct TerrainBundle {
    pub tile_atlas: Handle<TileAtlas>,
    #[cfg(feature = "high_precision")]
    pub cell: GridCell,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility_bundle: VisibilityBundle,
    pub no_frustum_culling: NoFrustumCulling,
}
