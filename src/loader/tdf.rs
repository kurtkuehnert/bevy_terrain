use std::path::Path;
use std::{fs, io};

const TDF_HEADER_SIZE: usize = 14;

#[derive(Debug)]
pub struct TDF {
    pub pixel_size: u32,
    pub channel_count: u32,
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
}

impl TDF {
    pub fn decode(data: &[u8]) -> (Self, Vec<u8>) {
        let mut header = Vec::from(data);
        let data = header.split_off(TDF_HEADER_SIZE);

        let descriptor = TDF {
            width: u32::from_be_bytes(header[0..4].try_into().unwrap()),
            height: u32::from_be_bytes(header[4..8].try_into().unwrap()),
            mip_count: u32::from_be_bytes(header[8..12].try_into().unwrap()),
            pixel_size: header[12] as u32,
            channel_count: header[13] as u32,
        };

        (descriptor, data)
    }

    pub fn load_file<P: AsRef<Path>>(path: P) -> io::Result<(Self, Vec<u8>)> {
        let mut header = fs::read(path)?;
        let data = header.split_off(TDF_HEADER_SIZE);

        let descriptor = TDF {
            width: u32::from_be_bytes(header[0..4].try_into().unwrap()),
            height: u32::from_be_bytes(header[4..8].try_into().unwrap()),
            mip_count: u32::from_be_bytes(header[8..12].try_into().unwrap()),
            pixel_size: header[12] as u32,
            channel_count: header[13] as u32,
        };

        Ok((descriptor, data))
    }

    pub fn save_file<P: AsRef<Path>>(&self, path: P, pixels: &[u8]) {
        let mut data = vec![0; TDF_HEADER_SIZE + pixels.len()];
        data[0..4].copy_from_slice(&self.width.to_be_bytes());
        data[4..8].copy_from_slice(&self.height.to_be_bytes());
        data[8..12].copy_from_slice(&self.mip_count.to_be_bytes());
        data[12] = self.pixel_size as u8;
        data[13] = self.channel_count as u8;
        data[14..].copy_from_slice(pixels);

        fs::write(path, data).unwrap();
    }
}
