//! Functions for preprocessing source tiles into streamable nodes.

pub mod attachment;
pub mod down_sample;
pub mod split;
pub mod stitch;

use crate::{
    preprocess::attachment::{preprocess_attachment, preprocess_base},
    terrain_data::{calc_node_id, AttachmentConfig, AttachmentFormat},
    TerrainConfig,
};
use bevy::prelude::*;
use bytemuck::cast_slice;
use image::{io::Reader, DynamicImage, ImageBuffer, ImageResult, Luma, LumaA, Rgb, Rgba};
use itertools::{Itertools, Product};
use std::fs::remove_file;
use std::{
    fs::{self, DirEntry, ReadDir},
    iter::Map,
    ops::Range,
    path::PathBuf,
};

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
pub(crate) struct BaseConfig {
    pub center_size: u32,
    pub border_size: u32,
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

pub(crate) fn reset_directory(directory: &str) {
    let _ = fs::remove_dir_all(directory);
    fs::create_dir_all(directory).unwrap();
}

pub(crate) fn format_directory(path: &str, name: &str) -> String {
    format!("assets/{path}/data/{name}")
}

pub(crate) fn format_node_path(directory: &str, lod: u32, x: u32, y: u32) -> String {
    let node_id = calc_node_id(lod, x, y);

    format!("{directory}/{node_id}.bin")
}

pub(crate) fn load_image(file_path: &str) -> ImageResult<DynamicImage> {
    let mut reader = Reader::open(file_path)?;
    reader.no_limits();
    reader.decode()
}

pub(crate) fn load_node(node_path: &str, attachment: &AttachmentConfig) -> Option<DynamicImage> {
    let size = attachment.texture_size();

    if let Ok(buffer) = fs::read(node_path) {
        let node_image = match attachment.format {
            AttachmentFormat::Rgb8 => {
                let image = Rgb8Image::from_raw(size, size, buffer).unwrap();
                DynamicImage::from(image)
            }
            AttachmentFormat::Rgba8 => {
                let image = Rgba8Image::from_raw(size, size, buffer).unwrap();
                DynamicImage::from(image)
            }
            AttachmentFormat::R16 => {
                let buffer = Vec::from(cast_slice(&buffer)); // Todo: improve this?
                let image = R16Image::from_raw(size, size, buffer).unwrap();
                DynamicImage::from(image)
            }
            AttachmentFormat::Rg16 => {
                let buffer = Vec::from(cast_slice(&buffer));
                let image = Rg16Image::from_raw(size, size, buffer).unwrap();
                DynamicImage::from(image)
            }
        };

        Some(node_image)
    } else {
        None
    }
}

pub(crate) fn load_or_create_node(node_path: &str, attachment: &AttachmentConfig) -> DynamicImage {
    if let Some(node_image) = load_node(node_path, attachment) {
        node_image
    } else {
        let size = attachment.texture_size();

        match attachment.format {
            AttachmentFormat::Rgb8 => DynamicImage::from(Rgb8Image::new(size, size)),
            AttachmentFormat::Rgba8 => DynamicImage::from(Rgba8Image::new(size, size)),
            AttachmentFormat::R16 => DynamicImage::from(R16Image::new(size, size)),
            AttachmentFormat::Rg16 => DynamicImage::from(Rg16Image::new(size, size)),
        }
    }
}

pub(crate) fn save_node(node_path: &str, node_image: &DynamicImage) {
    fs::write(node_path, node_image.as_bytes()).expect("Could not save node.");
}

pub(crate) fn convert_nodes(directory: &str, attachment: &AttachmentConfig) {
    for (_, node_path) in iterate_directory(directory).collect::<Vec<_>>() {
        let node_image = load_node(&node_path, attachment).unwrap();

        remove_file(&node_path).unwrap();

        let final_path = PathBuf::from(&node_path).with_extension("png");
        node_image.save(final_path).unwrap();
    }
}

pub(crate) fn iterate_directory(
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
