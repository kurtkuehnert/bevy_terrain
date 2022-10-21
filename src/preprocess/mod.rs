//! Functions for preprocessing source tiles into streamable nodes.

pub mod attachment;
pub mod config;
pub mod down_sample;
pub mod file_io;
pub mod split;
pub mod stitch;

use crate::preprocess::config::save_config;
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

#[macro_export]
macro_rules! return_none {
    ($res:expr) => {
        match $res {
            Some(val) => val,
            None => return,
        }
    };
}

#[derive(Copy, Clone)]
pub struct BaseConfig {
    pub texture_size: u32,
    pub mip_level_count: u32,
    pub file_format: FileFormat,
}

impl BaseConfig {
    pub fn new(texture_size: u32, mip_level_count: u32) -> Self {
        Self {
            texture_size,
            mip_level_count,
            file_format: FileFormat::TDF,
        }
    }

    pub(crate) fn height_attachment(&self) -> AttachmentConfig {
        let mut attachment = AttachmentConfig::new(
            "height".to_string(),
            self.texture_size,
            self.mip_level_count,
            AttachmentFormat::R16,
        );

        attachment.file_format = self.file_format;
        attachment
    }
    pub(crate) fn minmax_attachment(&self) -> AttachmentConfig {
        let mut attachment = AttachmentConfig::new(
            "minmax".to_string(),
            self.texture_size,
            self.mip_level_count,
            AttachmentFormat::Rg16,
        );

        attachment.file_format = self.file_format;
        attachment
    }
}

/// The configuration of the source tile(s) of an attachment.
#[derive(Default, Debug)]
pub struct TileConfig {
    /// The path to the tile/directory of tiles.
    pub path: String,
    /// The size of the tile in pixels.
    pub size: u32,
    /// The file format of the tile.
    pub file_format: FileFormat,
}

/// The preprocessor converts attachments from source data to streamable nodes.
///
/// It gathers all configurations of the attachments and then optionally processes them.
#[derive(Default)]
pub struct Preprocessor {
    pub(crate) base: Option<(TileConfig, BaseConfig)>,
    pub(crate) attachments: Vec<(TileConfig, AttachmentConfig)>,
}

impl Preprocessor {
    /// Preprocesses all attachments of the terrain.
    pub fn preprocess(self, config: &TerrainConfig) {
        if let Some(base) = self.base {
            preprocess_base(config, &base.0, &base.1);
        }

        for (tile, attachment) in self.attachments {
            preprocess_attachment(config, &tile, &attachment);
        }

        save_config(config);
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
        Itertools::cartesian_product(self.x..other.x, self.y..other.y)
    }
}

pub type Rgb8Image = ImageBuffer<Rgb<u8>, Vec<u8>>;
pub type Rgba8Image = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type R16Image = ImageBuffer<Luma<u16>, Vec<u16>>;
pub type Rg16Image = ImageBuffer<LumaA<u16>, Vec<u16>>;
