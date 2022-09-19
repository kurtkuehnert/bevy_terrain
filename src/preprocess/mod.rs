//! Functions for preprocessing source tiles into streamable nodes.

pub mod attachment;
pub mod down_sample;
pub mod file_io;
pub mod split;
pub mod stitch;

use crate::{
    preprocess::attachment::{preprocess_attachment, preprocess_base},
    terrain_data::{AttachmentConfig, AttachmentFormat, FileFormat},
    TerrainConfig,
};
use bevy::prelude::*;
use image::{ImageBuffer, Luma, LumaA, Rgb, Rgba};
use itertools::{Itertools, Product};
use std::ops::Range;

#[macro_export]
macro_rules! skip_none {
    ($res:expr) => {
        match $res {
            Some(val) => val,
            None => continue,
        }
    };
}

#[derive(Default)]
pub struct BaseConfig {
    pub center_size: u32,
    pub file_format: FileFormat,
}

impl BaseConfig {
    pub(crate) fn height_attachment(&self) -> AttachmentConfig {
        AttachmentConfig {
            name: "height".to_string(),
            center_size: self.center_size,
            border_size: 2,
            format: AttachmentFormat::R16,
            file_format: self.file_format,
        }
    }
    pub(crate) fn minmax_attachment(&self) -> AttachmentConfig {
        AttachmentConfig {
            name: "minmax".to_string(),
            center_size: self.center_size,
            border_size: 2,
            format: AttachmentFormat::Rg16,
            file_format: self.file_format,
        }
    }
}

/// The configuration of the source tile(s) of an attachment.
#[derive(Default)]
pub struct TileConfig {
    /// The path to the tile/directory of tiles.
    pub path: String,
    /// The lod of the tile.
    pub lod: u32,
    /// The offset of the tile.
    pub offset: UVec2,
    /// The size of the tile in pixels.
    pub size: u32,
}

/// The preprocessor converts attachments from source data to streamable nodes.
///
/// It gathers all configurations of the attachments and then optionally processes them.
#[derive(Default)]
pub struct Preprocessor {
    pub(crate) base: (TileConfig, BaseConfig),
    pub(crate) attachments: Vec<(TileConfig, AttachmentConfig)>,
}

impl Preprocessor {
    /// Preprocesses all attachments of the terrain.
    pub fn preprocess(self, config: &TerrainConfig) {
        preprocess_base(config, &self.base.0, &self.base.1);

        for (tile, attachment) in self.attachments {
            preprocess_attachment(config, &tile, &attachment);
        }
    }
}

pub(crate) trait UVec2Utils {
    fn div_floor(self, rhs: u32) -> Self;
    fn div_ceil(self, rhs: u32) -> Self;
    fn product(self, other: Self) -> Product<Range<u32>, Range<u32>>;
}

impl UVec2Utils for UVec2 {
    fn div_floor(self, rhs: u32) -> Self {
        self / rhs
    }

    fn div_ceil(self, rhs: u32) -> Self {
        (self + (rhs - 1)) / rhs
    }

    fn product(self, other: Self) -> Product<Range<u32>, Range<u32>> {
        Itertools::cartesian_product(self.x..other.y, self.y..other.y)
    }
}

pub(crate) type Rgb8Image = ImageBuffer<Rgb<u8>, Vec<u8>>;
pub(crate) type Rgba8Image = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub(crate) type R16Image = ImageBuffer<Luma<u16>, Vec<u16>>;
pub(crate) type Rg16Image = ImageBuffer<LumaA<u16>, Vec<u16>>;
