//! Types for configuring terrains.

use crate::{
    attachment_loader::{AttachmentFromDisk, AttachmentFromDiskLoader},
    preprocess::{BaseConfig, Preprocessor, TileConfig},
    terrain_data::{AtlasAttachment, AttachmentConfig, AttachmentFormat, AttachmentIndex},
};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::extract_component::ExtractComponent,
    utils::HashMap,
};

/// Resource that stores components that are associated to a terrain entity.
/// This is used to persist components in the render world.
/// Todo: replace this once the render world can persist components
pub(crate) type TerrainComponents<C> = HashMap<Entity, C>;

/// A marker component used to identify a terrain entity.
#[derive(Clone, Copy, Component)]
pub struct Terrain;

impl ExtractComponent for Terrain {
    type Query = Read<Self>;
    type Filter = ();

    #[inline]
    fn extract_component(_item: QueryItem<Self::Query>) -> Self {
        Self
    }
}

/// The configuration of a terrain.
///
/// Here you can define all fundamental parameters of the terrain.
#[derive(Clone, Component)]
pub struct TerrainConfig {
    /// The count of level of detail layers.
    pub lod_count: u32,
    /// The maximum height of the terrain. // Todo: reconsider this
    pub height: f32,
    /// The size of the smallest nodes.
    pub chunk_size: u32, // Todo: reconsider this
    /// The size of the terrain.
    pub terrain_size: u32, // Todo: reconsider this
    /// The amount of nodes the can be loaded simultaneously in the node atlas.
    pub node_atlas_size: u32,
    /// The path to the terrain folder inside the assets directory.
    pub path: String,
    /// The attachments of the terrain.
    pub attachments: Vec<AtlasAttachment>,
}

impl TerrainConfig {
    /// Creates a new terrain config without attachments.
    pub fn new(
        terrain_size: u32,
        chunk_size: u32,
        lod_count: u32,
        height: f32,
        node_atlas_size: u32,
        path: String,
    ) -> Self {
        Self {
            lod_count,
            height,
            node_atlas_size,
            chunk_size,
            terrain_size,
            path,
            attachments: default(),
        }
    }

    /// Adds an attachment to the terrain.
    ///
    /// The attachment will not be loaded automatically, but the caller has to handle the loading instead.
    pub fn add_attachment(&mut self, attachment: AttachmentConfig) -> AttachmentIndex {
        self.attachments.push(attachment.into());
        self.attachments.len() - 1
    }

    /// Adds an attachment to the terrain, which will be loaded from disk automatically.
    pub fn add_attachment_from_disk(
        &mut self,
        preprocessor: &mut Preprocessor,
        from_disk_loader: &mut AttachmentFromDiskLoader,
        attachment: AttachmentConfig,
        tile: TileConfig,
    ) {
        let attachment_index = self.add_attachment(attachment.clone());

        from_disk_loader.attachments.insert(
            attachment_index,
            AttachmentFromDisk {
                path: format!("{}/data/{}", self.path, attachment.name),
                format: attachment.format.into(),
            },
        );

        preprocessor.attachments.push((tile, attachment));
    }

    /// Adds the base attachment, which contains a height and density information.
    ///
    /// This is required by terrains, that use the default render pipeline.
    pub fn add_base_attachment(
        &mut self,
        preprocessor: &mut Preprocessor,
        from_disk_loader: &mut AttachmentFromDiskLoader,
        center_size: u32,
        tile: TileConfig,
    ) {
        let height_attachment = AttachmentConfig {
            name: "height".to_string(),
            center_size,
            border_size: 2,
            format: AttachmentFormat::LUMA16,
        };
        let density_attachment = AttachmentConfig {
            name: "density".to_string(),
            center_size,
            border_size: 0,
            format: AttachmentFormat::LUMA16,
        };

        preprocessor.base = (tile, BaseConfig { center_size });

        from_disk_loader.attachments.insert(
            self.attachments.len(),
            AttachmentFromDisk {
                path: format!("{}/data/{}", self.path, height_attachment.name),
                format: AttachmentFormat::LUMA16.into(),
            },
        );

        self.attachments.push(height_attachment.into());

        from_disk_loader.attachments.insert(
            self.attachments.len(),
            AttachmentFromDisk {
                path: format!("{}/data/{}", self.path, density_attachment.name),
                format: AttachmentFormat::LUMA16.into(),
            },
        );

        self.attachments.push(density_attachment.into());
    }
}
