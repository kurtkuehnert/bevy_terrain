pub mod attachment;
pub mod base;

use crate::{
    preprocess::{attachment::preprocess_attachment, base::preprocess_base},
    terrain_data::{calc_node_id, AttachmentConfig, AttachmentFormat},
    TerrainConfig,
};
use bevy::prelude::UVec2;
use image::{io::Reader, DynamicImage, ImageBuffer, ImageResult, Luma, RgbImage, RgbaImage};
use itertools::{Itertools, Product};
use std::fs;
use std::fs::{DirEntry, ReadDir};
use std::iter::Map;
use std::ops::Range;

#[macro_export]
macro_rules! skip_fail {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(_) => continue,
        }
    };
}

#[derive(Default)]
pub struct BaseConfig {
    pub center_size: u32,
}

#[derive(Default)]
pub struct TileConfig {
    pub path: String,
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

pub(crate) fn reset_directory(directory: &str) {
    let _ = fs::remove_dir_all(directory);
    fs::create_dir_all(directory).unwrap();
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

pub(crate) fn format_node_path(directory: &str, lod: u32, x: u32, y: u32) -> String {
    let node_id = calc_node_id(lod, x, y);

    format!("{directory}/{node_id}.png")
}

pub(crate) fn format_path(path: &str, name: &str) -> String {
    format!("assets/{path}/data/{name}")
}

pub(crate) fn load_image(file_path: &str) -> ImageResult<DynamicImage> {
    let mut reader = Reader::open(file_path)?;
    reader.no_limits();
    reader.decode()
}

pub(crate) fn load_or_create_node(node_path: &str, attachment: &AttachmentConfig) -> DynamicImage {
    if let Ok(output) = load_image(node_path) {
        output
    } else {
        let size = attachment.texture_size();

        match attachment.format {
            AttachmentFormat::RGB => DynamicImage::from(RgbImage::new(size, size)),
            AttachmentFormat::RGBA => DynamicImage::from(RgbaImage::new(size, size)),
            AttachmentFormat::LUMA16 => DynamicImage::from(LUMA16Image::new(size, size)),
        }
    }
}

pub(crate) fn iterate_images(
    directory: &str,
) -> Map<ReadDir, fn(std::io::Result<DirEntry>) -> (String, String)> {
    fs::read_dir(directory).unwrap().map(|path| {
        let path = path.unwrap().path();
        let name = path
            .with_extension("")
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let path = path.into_os_string().into_string().unwrap();

        (name, path)
    })
}
