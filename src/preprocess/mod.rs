pub mod attachment;
pub mod base;

use crate::{
    data_structures::{calc_node_id, AttachmentConfig, AttachmentFormat},
    preprocess::{attachment::preprocess_attachment, base::preprocess_base},
    TerrainConfig,
};
use bevy::prelude::UVec2;
use image::{io::Reader, DynamicImage, ImageBuffer, Luma, RgbImage, RgbaImage};
use itertools::{Itertools, Product};
use std::ops::Range;

macro_rules! skip_fail {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                warn!("An error: {}; skipped.", e);
                continue;
            }
        }
    };
}

#[derive(Default)]
pub struct BaseConfig {
    pub center_size: u32,
}

#[derive(Default)]
pub struct TileConfig {
    pub path: &'static str,
    pub lod: u32,
    pub offset: UVec2,
    pub size: u32,
}

#[derive(Default)]
pub struct Preprocessor {
    pub(crate) base: (TileConfig, BaseConfig),
    pub(crate) attachments: Vec<(TileConfig, AttachmentConfig)>,
}

impl Preprocessor {
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

pub(crate) type LUMA16Image = ImageBuffer<Luma<u16>, Vec<u16>>;

pub(crate) fn node_path(directory: &str, lod: u32, x: u32, y: u32) -> String {
    let node_id = calc_node_id(lod, x, y);

    format!("{directory}/{node_id}.png")
}

pub(crate) fn format_path(path: &str, name: &str) -> String {
    format!("assets/{path}/data/{name}")
}

pub(crate) fn read_image(file_path: &str) -> DynamicImage {
    let mut reader = Reader::open(file_path).unwrap();
    reader.no_limits();
    reader.decode().unwrap()
}

pub(crate) fn load_node(
    file_path: &str,
    center_size: u32,
    border_size: u32,
    format: AttachmentFormat,
) -> DynamicImage {
    if let Ok(output) = image::open(file_path) {
        output
    } else {
        let texture_size = center_size + 2 * border_size;

        match format {
            AttachmentFormat::RGB => DynamicImage::from(RgbImage::new(texture_size, texture_size)),
            AttachmentFormat::RGBA => {
                DynamicImage::from(RgbaImage::new(texture_size, texture_size))
            }
            AttachmentFormat::LUMA16 => {
                DynamicImage::from(LUMA16Image::new(texture_size, texture_size))
            }
        }
    }
}
