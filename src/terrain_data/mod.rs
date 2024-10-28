//! This module contains the two fundamental data structures of the terrain:
//! the [`TileTree`] and the [`TileAtlas`].
//!
//! # Explanation
//! Each terrain possesses one [`TileAtlas`], which can be configured
//! to store any [`AtlasAttachment`](tile_atlas::AtlasAttachment) required (eg. height, density, albedo, splat, edc.)
//! These attachments can vary in resolution and texture format.
//!
//! To decide which tiles should be currently loaded you can create multiple
//! [`TileTree`] views that correspond to one tile atlas.
//! These tile_trees request and release tiles from the tile atlas based on their quality
//! setting (`load_distance`).
//! Additionally they are then used to access the best loaded data at any position.
//!
//! Both the tile atlas and the tile_trees also have a corresponding GPU representation,
//! which can be used to access the terrain data in shaders.

use crate::util::CollectArray;
use bevy::{prelude::*, render::render_resource::*};
use bytemuck::cast_slice;
use itertools::{iproduct, Itertools};
use std::iter;

mod gpu_tile_atlas;
mod gpu_tile_tree;
mod tile_atlas;
mod tile_tree;

pub(crate) use crate::terrain_data::{
    gpu_tile_atlas::create_attachment_layout,
    tile_atlas::{AtlasAttachment, AtlasTile, AtlasTileAttachment, AtlasTileAttachmentWithData},
    tile_tree::TileTreeEntry,
};
pub use crate::terrain_data::{
    gpu_tile_atlas::GpuTileAtlas, gpu_tile_tree::GpuTileTree, tile_atlas::TileAtlas,
    tile_tree::TileTree,
};

pub const INVALID_ATLAS_INDEX: u32 = u32::MAX;
pub const INVALID_LOD: u32 = u32::MAX;

/// The data format of an attachment.
#[derive(Clone, Copy, Debug)]
pub enum AttachmentFormat {
    /// Three channels  8 bit unsigned integer
    RgbU8,
    /// Four channels  8 bit unsigned integer
    RgbaU8,
    /// One channel  16 bit unsigned integer
    RU16,
    /// One channel  16 bit integer
    RI16,
    /// Two channels 16 bit unsigned integer
    RgU16,
    /// One channel 32 bit float
    RF32,
}

impl AttachmentFormat {
    pub(crate) fn id(self) -> u32 {
        match self {
            AttachmentFormat::RgbU8 => 5,
            AttachmentFormat::RgbaU8 => 0,
            AttachmentFormat::RU16 => 1,
            AttachmentFormat::RgU16 => 3,
            AttachmentFormat::RF32 => 4,
            AttachmentFormat::RI16 => 6,
        }
    }
    pub(crate) fn render_format(self) -> TextureFormat {
        match self {
            AttachmentFormat::RgbU8 => TextureFormat::Rgba8UnormSrgb,
            AttachmentFormat::RgbaU8 => TextureFormat::Rgba8UnormSrgb,
            AttachmentFormat::RU16 => TextureFormat::R16Unorm,
            AttachmentFormat::RgU16 => TextureFormat::Rg16Unorm,
            AttachmentFormat::RF32 => TextureFormat::R32Float,
            AttachmentFormat::RI16 => TextureFormat::R16Snorm,
        }
    }

    pub(crate) fn processing_format(self) -> TextureFormat {
        match self {
            AttachmentFormat::RgbU8 => TextureFormat::Rgba8Unorm,
            AttachmentFormat::RgbaU8 => TextureFormat::Rgba8Unorm,
            _ => self.render_format(),
        }
    }

    pub(crate) fn pixel_size(self) -> u32 {
        match self {
            AttachmentFormat::RgbU8 => 4,
            AttachmentFormat::RgbaU8 => 4,
            AttachmentFormat::RU16 => 2,
            AttachmentFormat::RgU16 => 4,
            AttachmentFormat::RF32 => 4,
            AttachmentFormat::RI16 => 2,
        }
    }
}

/// Configures an attachment.
#[derive(Clone, Debug)]
pub struct AttachmentConfig {
    /// The name of the attachment.
    pub name: String,
    pub texture_size: u32,
    /// The overlapping border size around the tile, used to prevent sampling artifacts.
    pub border_size: u32,
    pub mip_level_count: u32,
    /// The format of the attachment.
    pub format: AttachmentFormat,
}

impl Default for AttachmentConfig {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            texture_size: 512,
            border_size: 1,
            mip_level_count: 1,
            format: AttachmentFormat::RU16,
        }
    }
}

#[derive(Clone)]
pub(crate) enum AttachmentData {
    None,
    /// Three channels  8 bit
    // Rgb8(Vec<(u8, u8, u8)>), Can not be represented currently
    /// Four  channels  8 bit
    RgbaU8(Vec<[u8; 4]>),
    /// One   channel  16 bit
    RU16(Vec<u16>),
    /// One   channel  16 bit
    RI16(Vec<i16>),
    /// Two   channels 16 bit
    RgU16(Vec<[u16; 2]>),
    RF32(Vec<f32>),
}

impl AttachmentData {
    pub(crate) fn from_bytes(data: &[u8], format: AttachmentFormat) -> Self {
        match format {
            AttachmentFormat::RgbU8 => Self::RgbaU8(
                data.chunks(3)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2], 255])
                    .collect_vec(),
            ),
            AttachmentFormat::RgbaU8 => Self::RgbaU8(cast_slice(data).to_vec()),
            AttachmentFormat::RU16 => Self::RU16(cast_slice(data).to_vec()),
            AttachmentFormat::RI16 => Self::RI16(cast_slice(data).to_vec()),
            AttachmentFormat::RgU16 => Self::RgU16(cast_slice(data).to_vec()),
            AttachmentFormat::RF32 => Self::RF32(cast_slice(data).to_vec()),
        }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        match self {
            AttachmentData::RgbaU8(data) => cast_slice(data),
            AttachmentData::RU16(data) => cast_slice(data),
            AttachmentData::RI16(data) => cast_slice(data),
            AttachmentData::RgU16(data) => cast_slice(data),
            AttachmentData::RF32(data) => cast_slice(data),
            AttachmentData::None => panic!("Attachment has no data."),
        }
    }

    pub(crate) fn generate_mipmaps(&mut self, texture_size: u32, mip_level_count: u32) {
        fn generate_mipmap_rgba8(
            data: &mut Vec<[u8; 4]>,
            parent_size: usize,
            child_size: usize,
            start: usize,
        ) {
            for (child_y, child_x) in iproduct!(0..child_size, 0..child_size) {
                let mut value = [0u64; 4];

                for i in 0..4 {
                    let parent_x = (child_x << 1) + (i >> 1);
                    let parent_y = (child_y << 1) + (i & 1);

                    let index = start + parent_y * parent_size + parent_x;

                    iter::zip(&mut value, data[index]).for_each(|(value, v)| *value += v as u64);
                }

                let value = value.iter().map(|value| (value / 4) as u8).collect_array();

                data.push(value);
            }
        }

        fn generate_mipmap_r16(
            data: &mut Vec<u16>,
            parent_size: usize,
            child_size: usize,
            start: usize,
        ) {
            for (child_y, child_x) in iproduct!(0..child_size, 0..child_size) {
                let mut value = 0;
                let mut count = 0;

                for (parent_x, parent_y) in
                    iproduct!(0..2, 0..2).map(|(x, y)| ((child_x << 1) + x, (child_y << 1) + y))
                {
                    let index = start + parent_y * parent_size + parent_x;
                    let data = data[index] as u32;

                    if data != 0 {
                        value += data;
                        count += 1;
                    }
                }

                let value = if count == 0 {
                    0
                } else {
                    (value / count) as u16
                };

                data.push(value);
            }
        }

        let mut start = 0;
        let mut parent_size = texture_size as usize;

        for _mip_level in 1..mip_level_count {
            let child_size = parent_size >> 1;

            match self {
                AttachmentData::RgbaU8(data) => {
                    generate_mipmap_rgba8(data, parent_size, child_size, start)
                }
                AttachmentData::RU16(data) => {
                    generate_mipmap_r16(data, parent_size, child_size, start)
                }
                _ => {}
            }

            start += parent_size * parent_size;
            parent_size = child_size;
        }
    }
}
