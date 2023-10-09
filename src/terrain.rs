//! Types for configuring terrains.

use crate::plugin::TerrainPluginConfig;
use crate::prelude::AttachmentConfig;
use crate::{
    attachment_loader::{AttachmentFromDisk, AttachmentFromDiskLoader},
    preprocess::{Preprocessor, TileConfig},
    terrain_data::{AtlasAttachment, NodeId},
};
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
    /// The size of the terrain.
    pub terrain_size: u32, // Todo: reconsider this
    /// The amount of nodes the can be loaded simultaneously in the node atlas.
    pub node_atlas_size: u32,
    /// The path to the terrain folder inside the assets directory.
    pub path: String,
    /// The attachments of the terrain.
    pub attachments: Vec<AtlasAttachment>,
    pub nodes: HashSet<NodeId>,
}

impl TerrainConfig {
    pub fn new(
        plugin_config: &TerrainPluginConfig,
        terrain_size: u32,
        lod_count: u32,
        height: f32,
        node_atlas_size: u32,
        path: String,
    ) -> Self {
        let attachments = plugin_config
            .attachments
            .clone()
            .into_iter()
            .map(AttachmentConfig::into)
            .collect();

        Self {
            lod_count,
            height,
            leaf_node_size: 0,
            terrain_size,
            node_atlas_size,
            path,
            attachments,
            nodes: HashSet::new(),
        }
    }

    pub fn add_base_attachment_from_disk(
        &mut self,
        plugin_config: &TerrainPluginConfig,
        preprocessor: &mut Preprocessor,
        loader: &mut AttachmentFromDiskLoader,
        tile: TileConfig,
    ) {
        self.leaf_node_size = plugin_config.leaf_node_size;

        loader.attachments.insert(
            0,
            AttachmentFromDisk::new(&plugin_config.base.height_attachment(), &self.path),
        );
        loader.attachments.insert(
            1,
            AttachmentFromDisk::new(&plugin_config.base.minmax_attachment(), &self.path),
        );

        preprocessor.base = Some((tile, plugin_config.base));
    }

    pub fn add_attachment_from_disk(
        &mut self,
        plugin_config: &TerrainPluginConfig,
        preprocessor: &mut Preprocessor,
        loader: &mut AttachmentFromDiskLoader,
        tile: TileConfig,
        attachment_index: usize,
    ) {
        let attachment = plugin_config.attachments[attachment_index].clone();

        loader.attachments.insert(
            attachment_index,
            AttachmentFromDisk::new(&attachment, &self.path),
        );

        preprocessor.attachments.push((tile, attachment));
    }
}
