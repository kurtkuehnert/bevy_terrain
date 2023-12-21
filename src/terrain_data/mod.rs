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
//! Additionally they are then used to access the best loaded data at any position.
//!
//! Both the node atlas and the quadtrees also have a corresponding GPU representation,
//! which can be used to access the terrain data in shaders.

use bevy::render::render_resource::*;
use bincode::{Decode, Encode};
use std::{fmt, str::FromStr};

pub mod gpu_node_atlas;
pub mod gpu_quadtree;
pub mod node_atlas;
pub mod quadtree;

#[cfg(feature = "spherical")]
pub const SIDE_COUNT: u32 = 6;
#[cfg(not(feature = "spherical"))]
pub const SIDE_COUNT: u32 = 1;

/// The global coordinate and identifier of a node.
#[derive(Copy, Clone, Default, Debug, Hash, Eq, PartialEq, ShaderType, Encode, Decode)]
pub struct NodeCoordinate {
    /// The side of the cube sphere the node is located on.
    pub side: u32,
    /// The lod of the node, where 0 is the highest level of detail with the smallest size
    /// and highest resolution
    pub lod: u32,
    /// The x position of the node in node sizes.
    pub x: u32,
    /// The y position of the node in node sizes.
    pub y: u32,
}

impl NodeCoordinate {
    pub fn new(side: u32, lod: u32, x: u32, y: u32) -> Self {
        Self { side, lod, x, y }
    }

    pub fn parent(self) -> Self {
        Self {
            side: self.side,
            lod: self.lod + 1,
            x: self.x >> 1,
            y: self.y >> 1,
        }
    }

    pub fn path(self, path: &str, extension: &str) -> String {
        format!("{path}/{self}.{extension}")
    }
}

impl fmt::Display for NodeCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}_{}_{}_{}", self.side, self.lod, self.x, self.y)
    }
}

impl FromStr for NodeCoordinate {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('_');

        Ok(Self {
            side: parts.next().unwrap().parse()?,
            lod: parts.next().unwrap().parse()?,
            x: parts.next().unwrap().parse()?,
            y: parts.next().unwrap().parse()?,
        })
    }
}

impl NodeCoordinate {
    const INVALID: NodeCoordinate = NodeCoordinate {
        side: u32::MAX,
        lod: u32::MAX,
        x: u32::MAX,
        y: u32::MAX,
    };
}

pub const INVALID_ATLAS_INDEX: u32 = u32::MAX;
pub const INVALID_LOD: u32 = u32::MAX;

/// The data format of an attachment.
#[derive(Encode, Decode, Clone, Copy, Debug)]
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

impl AttachmentFormat {
    pub(crate) fn sample_format(self) -> TextureFormat {
        match self {
            AttachmentFormat::Rgb8 => TextureFormat::Rgba8UnormSrgb,
            AttachmentFormat::Rgba8 => TextureFormat::Rgba8UnormSrgb,
            AttachmentFormat::R16 => TextureFormat::R16Unorm,
            AttachmentFormat::Rg16 => TextureFormat::Rg16Unorm,
        }
    }

    pub(crate) fn pixel_size(self) -> u32 {
        match self {
            AttachmentFormat::Rgb8 => 3,
            AttachmentFormat::Rgba8 => 4,
            AttachmentFormat::R16 => 2,
            AttachmentFormat::Rg16 => 4,
        }
    }
}

/// The file format used to store the terrain data.
#[derive(Encode, Decode, Clone, Copy, Debug)]
pub enum FileFormat {
    TDF,
    PNG,
    TIF,
    QOI,
    DTM,
}

impl Default for FileFormat {
    fn default() -> Self {
        Self::TDF
    }
}

impl FileFormat {
    pub(crate) fn extension(&self) -> &str {
        match self {
            Self::TDF => "tdf",
            Self::PNG => "png",
            Self::TIF => "tif",
            Self::QOI => "qoi",
            Self::DTM => "dtm",
        }
    }
}

/// Configures an attachment.
#[derive(Encode, Decode, Clone, Debug)]
pub struct AttachmentConfig {
    /// The name of the attachment.
    pub name: String,
    pub texture_size: u32,
    /// The none overlapping center size in pixels.
    pub center_size: u32,
    /// The overlapping border size around the node, used to prevent sampling artifacts.
    pub border_size: u32,
    pub mip_level_count: u32,
    /// The format of the attachment.
    pub format: AttachmentFormat,
    /// The file format of the attachment.
    pub file_format: FileFormat,
}

impl AttachmentConfig {
    pub fn new(
        name: String,
        texture_size: u32,
        border_size: u32,
        mip_level_count: u32,
        format: AttachmentFormat,
    ) -> Self {
        let center_size = texture_size - 2 * border_size;

        Self {
            name,
            texture_size,
            center_size,
            border_size,
            mip_level_count,
            format,
            file_format: FileFormat::TDF,
        }
    }
}
