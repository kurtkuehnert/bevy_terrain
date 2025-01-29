//! Types for configuring terrains.
//!

use crate::{
    math::{TerrainShape, TileCoordinate},
    terrain_data::{AttachmentConfig, AttachmentLabel},
};
use bevy::{ecs::entity::EntityHashMap, prelude::*, utils::HashMap};
use ron::error::SpannedResult;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

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
#[derive(Serialize, Deserialize, Asset, TypePath, Debug, Clone)]
pub struct TerrainConfig {
    /// The path to the terrain folder inside the assets directory.
    pub path: String,
    pub shape: TerrainShape,
    /// The count of level of detail layers.
    pub lod_count: u32,
    pub min_height: f32,
    pub max_height: f32,
    /// The attachments of the terrain.
    pub attachments: HashMap<AttachmentLabel, AttachmentConfig>,
    /// The tiles of the terrain.
    pub tiles: Vec<TileCoordinate>,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            shape: TerrainShape::Plane { side_length: 1.0 },
            lod_count: 1,
            min_height: 0.0,
            max_height: 1.0,
            path: default(),
            tiles: default(),
            attachments: default(),
        }
    }
}

impl TerrainConfig {
    pub fn add_attachment(
        &mut self,
        label: AttachmentLabel,
        attachment: AttachmentConfig,
    ) -> &mut Self {
        self.attachments.insert(label, attachment);
        self
    }

    pub fn load_file<P: AsRef<Path>>(path: P) -> SpannedResult<Self> {
        let encoded = fs::read_to_string(path)?;
        ron::from_str(&encoded)
    }

    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let encoded = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        fs::write(path, encoded)?;
        Ok(())
    }
}
