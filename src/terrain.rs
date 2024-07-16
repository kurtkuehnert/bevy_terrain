//! Types for configuring terrains.
//!
#[cfg(feature = "high_precision")]
use crate::big_space::{GridCell, GridTransformOwned, ReferenceFrame};

use crate::{
    math::TerrainModel,
    terrain_data::{tile_atlas::TileAtlas, AttachmentConfig},
};
use bevy::{
    ecs::entity::EntityHashMap,
    prelude::*,
    render::{extract_component::ExtractComponent, view::NoFrustumCulling},
};

/// Resource that stores components that are associated to a terrain entity.
/// This is used to persist components in the render world.
#[derive(Deref, DerefMut, Resource)]
pub struct TerrainComponents<C>(EntityHashMap<C>);

impl<C> Default for TerrainComponents<C> {
    fn default() -> Self {
        Self(default())
    }
}

/// A marker component used to identify a terrain entity.
#[derive(Clone, Copy, Component, ExtractComponent)]
pub struct Terrain;

/// The configuration of a terrain.
///
/// Here you can define all fundamental parameters of the terrain.
#[derive(Clone, Component, ExtractComponent)]
pub struct TerrainConfig {
    /// The count of level of detail layers.
    pub lod_count: u32,
    pub model: TerrainModel,
    /// The amount of tiles the can be loaded simultaneously in the tile atlas.
    pub tile_atlas_size: u32,
    /// The path to the terrain folder inside the assets directory.
    pub path: String,
    /// The attachments of the terrain.
    pub attachments: Vec<AttachmentConfig>,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            lod_count: 1,
            model: TerrainModel::sphere(default(), 1.0, 0.0, 1.0),
            tile_atlas_size: 1024,
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
#[derive(Bundle)]
pub struct TerrainBundle {
    pub terrain: Terrain,
    pub tile_atlas: TileAtlas,
    pub config: TerrainConfig,
    #[cfg(feature = "high_precision")]
    pub cell: GridCell,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility_bundle: VisibilityBundle,
    pub no_frustum_culling: NoFrustumCulling,
}

impl TerrainBundle {
    /// Creates a new terrain bundle from the config.
    #[cfg(feature = "high_precision")]
    pub fn new(config: TerrainConfig, frame: &ReferenceFrame) -> Self {
        let GridTransformOwned { transform, cell } = config.model.grid_transform(frame);

        Self {
            terrain: Terrain,
            tile_atlas: TileAtlas::from_config(&config),
            transform,
            config,
            cell,
            global_transform: default(),
            visibility_bundle: VisibilityBundle {
                visibility: Visibility::Visible,
                inherited_visibility: default(),
                view_visibility: default(),
            },
            no_frustum_culling: NoFrustumCulling,
        }
    }

    #[cfg(not(feature = "high_precision"))]
    pub fn new(config: TerrainConfig) -> Self {
        let transform = config.model.transform();

        Self {
            terrain: Terrain,
            tile_atlas: TileAtlas::from_config(&config),
            transform,
            config,
            global_transform: default(),
            visibility_bundle: VisibilityBundle {
                visibility: Visibility::Visible,
                inherited_visibility: default(),
                view_visibility: default(),
            },
            no_frustum_culling: NoFrustumCulling,
        }
    }
}
