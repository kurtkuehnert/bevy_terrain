use bevy::render::render_resource::TextureFormat;
use bytemuck::cast_slice;
use itertools::{Itertools, iproduct};
use serde::{Deserialize, Serialize};
use std::{fmt::Error, iter, str::FromStr};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub enum AttachmentLabel {
    #[default]
    Height,
    Custom(String), // Todo: this should not be a heap allocated string
    Empty(usize),
}

impl From<&AttachmentLabel> for String {
    fn from(value: &AttachmentLabel) -> Self {
        match value {
            AttachmentLabel::Height => "height".to_string(),
            AttachmentLabel::Custom(name) => name.clone(),
            AttachmentLabel::Empty(i) => format!("empty{i}").to_string(),
        }
    }
}

impl FromStr for AttachmentLabel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "height" => Ok(Self::Height),
            name => Ok(Self::Custom(name.to_string())),
        }
    }
}

/// The data format of an attachment.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
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

impl FromStr for AttachmentFormat {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "rgu8" => Ok(Self::RgbU8),
            "rgbau8" => Ok(Self::RgbaU8),
            "ru16" => Ok(Self::RU16),
            "ri16" => Ok(Self::RI16),
            "rf32" => Ok(Self::RF32),
            _ => Err(Error),
        }
    }
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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AttachmentConfig {
    /// The name of the attachment.
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
            texture_size: 512,
            border_size: 1,
            mip_level_count: 1,
            format: AttachmentFormat::RU16,
        }
    }
}

impl AttachmentConfig {
    pub fn center_size(&self) -> u32 {
        self.texture_size - 2 * self.border_size
    }

    pub fn offset_size(&self) -> u32 {
        self.texture_size - self.border_size
    }
}

#[derive(Clone)]
pub enum AttachmentData {
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
            //  AttachmentData::None => panic!("Attachment has no data."),
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

                let value = value
                    .iter()
                    .map(|value| (value / 4) as u8)
                    .collect_array()
                    .unwrap();

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

        fn generate_mipmap_f32(
            data: &mut Vec<f32>,
            parent_size: usize,
            child_size: usize,
            start: usize,
        ) {
            for (child_y, child_x) in iproduct!(0..child_size, 0..child_size) {
                let mut value = 0.0;
                let mut count = 0;

                for (parent_x, parent_y) in
                    iproduct!(0..2, 0..2).map(|(x, y)| ((child_x << 1) + x, (child_y << 1) + y))
                {
                    let index = start + parent_y * parent_size + parent_x;
                    let data = data[index];

                    if data != 0.0 {
                        value += data;
                        count += 1;
                    }
                }

                let value = if count == 0 {
                    0.0
                } else {
                    value / count as f32
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
                AttachmentData::RF32(data) => {
                    generate_mipmap_f32(data, parent_size, child_size, start)
                }
                _ => {
                    unimplemented!()
                }
            }

            start += parent_size * parent_size;
            parent_size = child_size;
        }
    }
}
