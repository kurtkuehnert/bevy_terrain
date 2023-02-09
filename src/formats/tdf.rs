use anyhow::{anyhow, Result};
use dtm::DTM;
use itertools::iproduct;
use rapid_qoi::{Colors, Qoi};
use std::{fs, path::Path};

const TDF_HEADER_SIZE: usize = 7;

#[derive(Debug)]
pub struct TDF {
    pub pixel_size: u32,
    pub channel_count: u32,
    pub mip_level_count: u32,
    pub size: u32,
}

impl TDF {
    fn decoded_size(&self, mip_level: u32) -> usize {
        let size = self.size >> mip_level;

        (size * size * self.pixel_size * self.channel_count) as usize
    }

    pub fn decode_alloc(encoded: &[u8], mip_maps: bool) -> Result<(Self, Vec<u8>)> {
        let mut descriptor = TDF {
            pixel_size: encoded[0] as u32,
            channel_count: encoded[1] as u32,
            mip_level_count: encoded[2] as u32,
            size: u32::from_be_bytes(encoded[3..7].try_into().unwrap()),
        };

        if !mip_maps {
            descriptor.mip_level_count = 1;
        }

        let mut total_decoded_size = 0;

        for mip_level in 0..descriptor.mip_level_count {
            total_decoded_size += descriptor.decoded_size(mip_level);
        }

        let mut decoded = vec![0; total_decoded_size];

        {
            let encoded_start = TDF_HEADER_SIZE;
            let decoded_start = 0;
            let encoded_size = encoded.len() - TDF_HEADER_SIZE;
            let decoded_size = descriptor.decoded_size(0);

            let encoded = &encoded[encoded_start..encoded_start + encoded_size];
            let decoded = &mut decoded[decoded_start..decoded_start + decoded_size];

            if encoded_size == decoded_size {
                decoded.copy_from_slice(encoded);
            } else if descriptor.pixel_size == 1 {
                Qoi::decode(encoded, decoded)?;
            } else if descriptor.pixel_size == 2 {
                DTM::decode(encoded, decoded)?;
            }
        }

        let mut decoded_start = 0;

        for mip_level in 1..descriptor.mip_level_count {
            let decoded_size = descriptor.decoded_size(mip_level - 1);

            let p_size = (descriptor.size >> (mip_level - 1)) as usize;
            let c_size = (descriptor.size >> mip_level) as usize;
            let p_start = decoded_start;
            let c_start = decoded_start + decoded_size;

            match (descriptor.channel_count, descriptor.pixel_size) {
                (1, 2) => generate_mipmap::<1, 2>(&mut decoded, p_size, c_size, p_start, c_start),
                (2, 2) => generate_mipmap::<2, 2>(&mut decoded, p_size, c_size, p_start, c_start),
                (3, 1) => generate_mipmap::<3, 1>(&mut decoded, p_size, c_size, p_start, c_start),
                (4, 1) => generate_mipmap::<4, 1>(&mut decoded, p_size, c_size, p_start, c_start),
                (_, _) => {}
            }

            decoded_start += decoded_size;
        }

        Ok((descriptor, decoded))
    }

    pub fn encode_alloc(&self, decoded: &[u8]) -> Result<Vec<u8>> {
        let mut decoded_size = 0;

        for mip_level in 0..self.mip_level_count {
            decoded_size += self.decoded_size(mip_level);
        }

        let mut encoded = vec![0; TDF_HEADER_SIZE + decoded_size];

        encoded[0] = self.pixel_size as u8;
        encoded[1] = self.channel_count as u8;
        encoded[2] = self.mip_level_count as u8;
        encoded[3..7].copy_from_slice(&self.size.to_be_bytes());

        let decoded_start = 0;
        let encoded_start = TDF_HEADER_SIZE;

        let decoded_size = self.decoded_size(0);
        let mut encoded_size = decoded_size;

        let decoded = &decoded[decoded_start..decoded_start + decoded_size];

        if self.pixel_size == 1 {
            let colors = match self.channel_count {
                3 => Ok(Colors::Rgb),
                4 => Ok(Colors::Rgba),
                _ => Err(anyhow!("Invalid data.")),
            }?;

            let descriptor = Qoi {
                width: self.size,
                height: self.size,
                colors,
            };

            let qoi_encoded = descriptor.encode_alloc(decoded)?;

            if qoi_encoded.len() < encoded_size {
                encoded_size = qoi_encoded.len();
                encoded[encoded_start..encoded_start + encoded_size].copy_from_slice(&qoi_encoded);
            }
        } else if self.pixel_size == 2 {
            let descriptor = DTM {
                pixel_size: self.pixel_size,
                channel_count: self.channel_count,
                width: self.size,
                height: self.size,
            };

            let dtm_encoded = descriptor.encode_alloc(decoded)?;

            if dtm_encoded.len() < encoded_size {
                encoded_size = dtm_encoded.len();
                encoded[encoded_start..encoded_start + encoded_size].copy_from_slice(&dtm_encoded);
            }
        }

        if encoded_size == decoded_size {
            encoded[encoded_start..encoded_start + encoded_size].copy_from_slice(decoded);
        }

        encoded.truncate(encoded_start + encoded_size);

        Ok(encoded)
    }

    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<(Self, Vec<u8>)> {
        let encoded = fs::read(path)?;
        Self::decode_alloc(&encoded, false)
    }

    pub fn save_file<P: AsRef<Path>>(&self, path: P, decoded: &[u8]) -> Result<()> {
        let encoded = self.encode_alloc(decoded)?;

        fs::write(path, encoded)?;
        Ok(())
    }
}

fn generate_mipmap<const C: usize, const P: usize>(
    decoded: &mut [u8],
    p_size: usize,
    c_size: usize,
    p_start: usize,
    c_start: usize,
) {
    for (c_y, c_x, c) in iproduct!(0..c_size, 0..c_size, 0..C) {
        let mut value = 0;

        for i in 0..4 {
            let p_x = (c_x << 1) + (i >> 1);
            let p_y = (c_y << 1) + (i & 1);

            let index = p_start + P * (C * (p_y * p_size + p_x) + c);

            for (j, &byte) in decoded[index..index + P].iter().enumerate() {
                value += (byte as u64) << ((j as u64) << 3);
            }
        }

        value /= 4;

        let index = c_start + P * (C * (c_y * c_size + c_x) + c);

        for (j, byte) in decoded[index..index + P].iter_mut().enumerate() {
            *byte = ((value >> ((j as u64) << 3)) & 0xFF) as u8;
        }
    }
}
