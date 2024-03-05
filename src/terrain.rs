//! Types for configuring terrains.

use crate::{terrain_data::node_atlas::NodeAtlas, terrain_data::AttachmentConfig};
use bevy::{
    prelude::*, render::extract_component::ExtractComponent, render::view::NoFrustumCulling,
    utils::HashMap,
};

/// Resource that stores components that are associated to a terrain entity.
/// This is used to persist components in the render world.
/// Todo: replace this once the render world can persist components
#[derive(Clone, Resource)]
pub struct TerrainComponents<C>(pub HashMap<Entity, C>);

impl<C> TerrainComponents<C> {
    pub fn get(&self, k: &Entity) -> Option<&C> {
        self.0.get(k)
    }

    pub fn get_mut(&mut self, k: &Entity) -> Option<&mut C> {
        self.0.get_mut(k)
    }

    pub fn insert(&mut self, k: Entity, v: C) {
        self.0.insert(k, v);
    }
}

impl<C> FromWorld for TerrainComponents<C> {
    fn from_world(_world: &mut World) -> Self {
        Self(default())
    }
}

/// A marker component used to identify a terrain entity.
#[derive(Clone, Copy, Component, ExtractComponent)]
pub struct Terrain;

/// The configuration of a terrain.
///
/// Here you can define all fundamental parameters of the terrain.
#[derive(Clone, Component)]
pub struct TerrainConfig {
    /// The count of level of detail layers.
    pub lod_count: u32,
    /// The minimum height of the terrain.
    pub min_height: f32,
    /// The maximum height of the terrain.
    pub max_height: f32,
    /// The amount of nodes the can be loaded simultaneously in the node atlas.
    pub node_atlas_size: u32,
    /// The path to the terrain folder inside the assets directory.
    pub path: String,
    /// The attachments of the terrain.
    pub attachments: Vec<AttachmentConfig>,
    pub attachment_groups: Vec<Vec<u32>>,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            lod_count: 1,
            min_height: 0.0,
            max_height: 1.0,
            node_atlas_size: 1024,
            path: default(),
            attachments: default(),
            attachment_groups: default(),
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
    pub node_atlas: NodeAtlas,
    pub config: TerrainConfig,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility_bundle: VisibilityBundle,
    pub no_frustum_culling: NoFrustumCulling,
}

impl TerrainBundle {
    /// Creates a new terrain bundle from the config.
    pub fn new(config: TerrainConfig, translation: Vec3, scale: f32) -> Self {
        Self {
            terrain: Terrain,
            node_atlas: NodeAtlas::from_config(&config),
            config,
            transform: Transform {
                translation,
                scale: Vec3::splat(scale),
                ..default()
            },
            global_transform: default(),
            visibility_bundle: default(),
            no_frustum_culling: NoFrustumCulling,
        }
    }
}
