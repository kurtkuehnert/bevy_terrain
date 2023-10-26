//! Types for configuring terrains.

use crate::terrain_data::{AtlasAttachment, NodeCoordinate};
use bevy::{
    prelude::*,
    render::extract_component::ExtractComponent,
    utils::{HashMap, HashSet},
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
    /// The maximum height of the terrain. // Todo: reconsider this
    pub height: f32,
    /// The size of the smallest nodes (with lod 0).
    pub leaf_node_size: u32, // Todo: reconsider this
    pub leaf_node_count: f32,
    /// The size of the terrain.
    pub terrain_size: f32, // Todo: reconsider this
    pub radius: f32,
    /// The amount of nodes the can be loaded simultaneously in the node atlas.
    pub node_atlas_size: u32,
    /// The path to the terrain folder inside the assets directory.
    pub path: String,
    /// The attachments of the terrain.
    pub attachments: Vec<AtlasAttachment>,
    pub nodes: HashSet<NodeCoordinate>,
}
