//! Contains the implementation for preprocessing source tiles into streamable nodes.

pub mod down_sample;
pub mod file_io;
pub mod split;
pub mod stitch;

use crate::{
    formats::tc::save_node_config,
    preprocess::{
        down_sample::{down_sample_layer, linear},
        file_io::{format_directory, reset_directory},
        split::split_tiles,
        stitch::stitch_layer,
    },
    terrain_data::{AttachmentConfig, AttachmentFormat, FileFormat},
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

/// The configuration of the base attachment of the terrain.
/// The base attachment consists of the height data of the terrain.
#[derive(Copy, Clone)]
pub struct BaseConfig {
    pub texture_size: u32,
    pub border_size: u32,
    pub mip_level_count: u32,
    pub file_format: FileFormat,
}

impl BaseConfig {
    pub fn new(texture_size: u32, mip_level_count: u32) -> Self {
        Self {
            texture_size,
            border_size: 2,
            mip_level_count,
            file_format: FileFormat::PNG,
        }
    }

    pub(crate) fn height_attachment(&self) -> AttachmentConfig {
        let mut attachment = AttachmentConfig::new(
            "height".to_string(),
            self.texture_size,
            self.border_size,
            self.mip_level_count,
            AttachmentFormat::R16,
        );

        attachment.file_format = self.file_format;
        attachment
    }
}

/// The configuration of the source tile(s) of an attachment.
#[derive(Clone, Default, Debug)]
pub struct TileConfig {
    /// The path to the tile/directory of tiles.
    pub path: String,
    /// The size of the tile in pixels.
    pub size: u32,
    pub side: u32,
    /// The file format of the tile.
    pub file_format: FileFormat,
}

/// The preprocessor converts attachments from source data to streamable nodes.
///
/// It gathers all configurations of the attachments and then optionally processes them.
pub struct OldPreprocessor {
    lod_count: u32,
    path: String,
    pub(crate) base: Option<(TileConfig, BaseConfig)>,
    pub(crate) attachments: Vec<(TileConfig, AttachmentConfig)>,
}

impl OldPreprocessor {
    pub fn new(lod_count: u32, path: String) -> Self {
        Self {
            lod_count,
            path,
            base: None,
            attachments: vec![],
        }
    }

    /// Preprocesses all attachments of the terrain.
    pub fn preprocess(&self) {
        if let Some((tile, base)) = &self.base {
            self.preprocess_base(tile, base);
        }

        for (tile, attachment) in &self.attachments {
            self.preprocess_attachment(tile, attachment);
        }

        save_node_config(&self.path);
    }

    fn preprocess_base(&self, tile: &TileConfig, base: &BaseConfig) {
        let height_attachment = base.height_attachment();

        let height_directory = format_directory(&self.path, "height");

        reset_directory(&height_directory);

        let (mut first, mut last) = split_tiles(&height_directory, tile, &height_attachment);

        for lod in 1..self.lod_count {
            first = first.div_floor(2);
            last = last.div_ceil(2);

            down_sample_layer(
                linear,
                &height_directory,
                &height_attachment,
                lod,
                first,
                last,
            );
            stitch_layer(&height_directory, &height_attachment, lod, first, last);
        }
    }

    fn preprocess_attachment(&self, tile: &TileConfig, attachment: &AttachmentConfig) {
        let directory = format_directory(&self.path, &attachment.name);

        reset_directory(&directory);

        let (mut first, mut last) = split_tiles(&directory, tile, attachment);

        for lod in 1..self.lod_count {
            first = first.div_floor(2);
            last = last.div_ceil(2);

            down_sample_layer(linear, &directory, attachment, lod, first, last);
            stitch_layer(&directory, attachment, lod, first, last);
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
        Itertools::cartesian_product(self.x..other.x, self.y..other.y)
    }
}

pub type Rgb8Image = ImageBuffer<Rgb<u8>, Vec<u8>>;
pub type Rgba8Image = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type R16Image = ImageBuffer<Luma<u16>, Vec<u16>>;
pub type Rg16Image = ImageBuffer<LumaA<u16>, Vec<u16>>;
