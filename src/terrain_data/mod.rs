//! This module contains the two fundamental data structures of the terrain:
//! the [`Quadtree`](quadtree::Quadtree) and the [`NodeAtlas`](node_atlas::NodeAtlas).
//!
//! # Explanation
//! Each terrain possesses one [`NodeAtlas`](node_atlas::NodeAtlas), which can be configured
//! to store any [`AtlasAttachment`] required (eg. height, density, albedo, splat, edc.)
//! These attachments can vary in resolution and texture format.
//!
//! To decide which nodes should be currently loaded you can create multiple
//! [`Quadtree`](quadtree::Quadtree) views that correspond to one node atlas.
//! These quadtrees request and release nodes from the node atlas based on their quality
//! setting (`load_distance`).
//! Additionally they are also used to access the best loaded data at any position.
//!
//! Both the node atlas and the quadtrees also have a corresponding GPU representation,
//! which can be used to access the terrain data in shaders.

use bevy::{prelude::*, render::render_resource::*, utils::Uuid};
use std::str::FromStr;

pub mod gpu_node_atlas;
pub mod gpu_quadtree;
pub mod node_atlas;
pub mod quadtree;

// Todo: may be swap to u64 for giant terrains
// Todo: consider 3 bit face data, for cube sphere
/// A globally unique identifier of a node.
/// lod |  x |  y
///   4 | 14 | 14
pub type NodeId = u32;
pub const INVALID_NODE_ID: NodeId = NodeId::MAX;

/// Identifier of a node (and its attachments) inside the node atlas.
pub type AtlasIndex = u16;
pub const INVALID_ATLAS_INDEX: AtlasIndex = AtlasIndex::MAX;

pub const INVALID_LOD: u16 = u16::MAX;

/// Identifier of an attachment inside the node atlas.
pub type AttachmentIndex = usize;

/// The global coordinate of a node.
pub struct NodeCoordinate {
    /// The lod of the node, where 0 is the highest level of detail with the smallest size
    /// and highest resolution
    pub lod: u32,
    /// The x position of the node in node sizes.
    pub x: u32,
    /// The y position of the node in node sizes.
    pub y: u32,
}

impl From<NodeId> for NodeCoordinate {
    /// Determines the coordinate of the node based on its id.
    #[inline]
    fn from(id: NodeId) -> Self {
        Self {
            lod: ((id >> 28) & 0xF) as u32,
            x: ((id >> 14) & 0x3FFF) as u32,
            y: (id & 0x3FFF) as u32,
        }
    }
}

/// Calculates the node identifier from the node coordinate.
#[inline]
pub fn calc_node_id(lod: u32, x: u32, y: u32) -> NodeId {
    (lod as NodeId & 0xF) << 28 | (x as NodeId & 0x3FFF) << 14 | y as NodeId & 0x3FFF
}

/// The data format of an attachment.
#[derive(Clone, Copy, Debug)]
pub enum AttachmentFormat {
    /// Three channels  8 bit
    Rgb8,
    /// Four  channels  8 bit
    Rgba8,
    /// One   channel  16 bit
    R16,
    /// Two   channels 16 bit
    Rg16,
}

impl From<AttachmentFormat> for TextureFormat {
    fn from(format: AttachmentFormat) -> Self {
        match format {
            AttachmentFormat::Rgb8 => TextureFormat::Rgba8UnormSrgb,
            AttachmentFormat::Rgba8 => TextureFormat::Rgba8UnormSrgb,
            AttachmentFormat::R16 => TextureFormat::R16Unorm,
            AttachmentFormat::Rg16 => TextureFormat::Rg16Unorm,
        }
    }
}

/// The file format used to store the terrain data.
#[derive(Clone, Copy, Debug)]
pub enum FileFormat {
    BIN,
    PNG,
    TIF,
    QOI,
    DTM,
}

impl Default for FileFormat {
    fn default() -> Self {
        Self::BIN
    }
}

impl FileFormat {
    pub(crate) fn extension(&self) -> &str {
        match self {
            Self::BIN => "bin",
            Self::PNG => "png",
            Self::TIF => "tif",
            Self::QOI => "qoi",
            Self::DTM => "dtm",
        }
    }
}

/// Configures an attachment.
#[derive(Clone)]
pub struct AttachmentConfig {
    /// The name of the attachment.
    pub name: String,
    /// The none overlapping center size in pixels.
    pub center_size: u32,
    /// The overlapping border size around the node, used to prevent sampling artifacts.
    pub border_size: u32,
    /// The format of the attachment.
    pub format: AttachmentFormat,
    /// The file format of the attachment.
    pub file_format: FileFormat,
}

impl AttachmentConfig {
    pub fn texture_size(&self) -> u32 {
        self.center_size + 2 * self.border_size
    }
}

/// An attachment of a [`NodeAtlas`](node_atlas::NodeAtlas).
#[derive(Clone)]
pub struct AtlasAttachment {
    /// The handle of the attachment array texture.
    pub(crate) handle: Handle<Image>,
    /// The name of the attachment.
    pub(crate) name: String,
    /// The none overlapping center size in pixels.
    pub(crate) center_size: u32,
    /// The overlapping border size around the node, used to prevent sampling artifacts.
    pub(crate) border_size: u32,
    /// The format of the attachment.
    pub(crate) format: TextureFormat,
}

impl From<AttachmentConfig> for AtlasAttachment {
    fn from(config: AttachmentConfig) -> Self {
        // Todo: fix this awful hack
        let handle = HandleUntyped::weak_from_u64(
            Uuid::from_str("6ea26da6-6cf8-4ea2-9986-1d7bf6c17d6f").unwrap(),
            fastrand::u64(..),
        )
        .typed();

        Self {
            handle,
            name: config.name,
            center_size: config.center_size,
            border_size: config.border_size,
            format: config.format.into(),
        }
    }
}
