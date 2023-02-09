use crate::{
    formats::tdf::TDF,
    preprocess::{R16Image, Rg16Image, Rgb8Image, Rgba8Image},
    terrain_data::{calc_node_id, AttachmentConfig, AttachmentFormat, FileFormat},
};
use bytemuck::cast_slice;
use dtm::DTM;
use image::{io::Reader, DynamicImage};
use rapid_qoi::{Colors, Qoi};
use std::{
    fs::{self, DirEntry, ReadDir},
    iter::FilterMap,
    path::Path,
};

#[allow(clippy::type_complexity)]
pub(crate) fn iterate_directory(
    directory: &str,
) -> FilterMap<ReadDir, fn(std::io::Result<DirEntry>) -> Option<(String, String)>> {
    fs::read_dir(directory).unwrap().filter_map(|path| {
        let path = path.unwrap().path();

        let name = path
            .with_extension("")
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let path = path.into_os_string().into_string().unwrap();

        if name.starts_with('.') {
            None
        } else {
            Some((name, path))
        }
    })
}

pub fn reset_directory(directory: &str) {
    let _ = fs::remove_dir_all(directory);
    fs::create_dir_all(directory).unwrap();
}

pub(crate) fn format_directory(path: &str, name: &str) -> String {
    if path.starts_with('/') {
        format!("{path}/data/{name}")
    } else {
        format!("assets/{path}/data/{name}")
    }
}

pub(crate) fn format_node_path(directory: &str, lod: u32, x: u32, y: u32) -> String {
    let node_id = calc_node_id(lod, x, y);

    format!("{directory}/{node_id}",)
}

pub fn load_image(path: &str, file_format: FileFormat) -> Option<DynamicImage> {
    let path = Path::new(path);
    let path = path.with_extension(file_format.extension());
    let path = path.to_str().unwrap();

    match file_format {
        FileFormat::TDF => load_tdf(path),
        FileFormat::PNG | FileFormat::TIF => load_image_rs(path),
        FileFormat::QOI => load_qoi(path),
        FileFormat::DTM => load_dtm(path),
    }
}

pub(crate) fn load_or_create_node(path: &str, attachment: &AttachmentConfig) -> DynamicImage {
    if let Some(node_image) = load_image(path, attachment.file_format) {
        node_image
    } else {
        let size = attachment.texture_size;

        match attachment.format {
            AttachmentFormat::Rgb8 => DynamicImage::from(Rgb8Image::new(size, size)),
            AttachmentFormat::Rgba8 => DynamicImage::from(Rgba8Image::new(size, size)),
            AttachmentFormat::R16 => DynamicImage::from(R16Image::new(size, size)),
            AttachmentFormat::Rg16 => DynamicImage::from(Rg16Image::new(size, size)),
        }
    }
}

pub fn save_image(path: &str, node_image: &DynamicImage, attachment: &AttachmentConfig) {
    let path = Path::new(path);
    let path = path.with_extension(attachment.file_format.extension());
    let path = path.to_str().unwrap();

    match attachment.file_format {
        FileFormat::TDF => save_tdf(path, node_image, attachment),
        FileFormat::PNG | FileFormat::TIF => save_image_rs(path, node_image, attachment),
        FileFormat::QOI => save_qoi(path, node_image, attachment),
        FileFormat::DTM => save_dtm(path, node_image, attachment),
    }
}

fn load_tdf(path: &str) -> Option<DynamicImage> {
    let (descriptor, data) = TDF::load_file(path).ok()?;
    let size = descriptor.size;

    match (descriptor.pixel_size, descriptor.channel_count) {
        (1, 3) => {
            let image = Rgb8Image::from_raw(size, size, data).unwrap();
            Some(DynamicImage::from(image))
        }
        (1, 4) => {
            let image = Rgba8Image::from_raw(size, size, data).unwrap();
            Some(DynamicImage::from(image))
        }
        (2, 1) => {
            let data: Vec<u16> = data
                .chunks_exact(2)
                .map(|pixel| u16::from_ne_bytes(pixel.try_into().unwrap()))
                .collect();

            let image = R16Image::from_raw(size, size, data).unwrap();
            Some(DynamicImage::from(image))
        }
        (2, 2) => {
            let data: Vec<u16> = data
                .chunks_exact(2)
                .map(|pixel| u16::from_le_bytes(pixel.try_into().unwrap()))
                .collect();

            let image = Rg16Image::from_raw(size, size, data).unwrap();
            Some(DynamicImage::from(image))
        }
        _ => None,
    }
}

fn load_image_rs(path: &str) -> Option<DynamicImage> {
    let mut reader = Reader::open(path).ok()?;
    reader.no_limits();
    Some(reader.decode().unwrap())
}

fn load_dtm(path: &str) -> Option<DynamicImage> {
    let (descriptor, data) = DTM::decode_file(path).ok()?;

    match descriptor.channel_count {
        1 => {
            let data: Vec<u16> = data
                .chunks_exact(2)
                .map(|pixel| u16::from_le_bytes(pixel.try_into().unwrap()))
                .collect();

            let image = R16Image::from_raw(descriptor.width, descriptor.height, data).unwrap();
            Some(DynamicImage::from(image))
        }
        2 => {
            let data: Vec<u16> = data
                .chunks_exact(2)
                .map(|pixel| u16::from_le_bytes(pixel.try_into().unwrap()))
                .collect();

            let image = Rg16Image::from_raw(descriptor.width, descriptor.height, data).unwrap();
            Some(DynamicImage::from(image))
        }
        _ => None,
    }
}

fn load_qoi(path: &str) -> Option<DynamicImage> {
    let bytes = fs::read(path).ok()?;
    let (descriptor, pixels) = Qoi::decode_alloc(&bytes).unwrap();

    match descriptor.colors {
        Colors::Rgb => {
            let image = Rgb8Image::from_raw(descriptor.width, descriptor.height, pixels).unwrap();
            Some(DynamicImage::from(image))
        }
        Colors::Rgba => {
            let image = Rgba8Image::from_raw(descriptor.width, descriptor.height, pixels).unwrap();
            Some(DynamicImage::from(image))
        }
        _ => None,
    }
}

fn save_tdf(path: &str, node_image: &DynamicImage, attachment: &AttachmentConfig) {
    let (pixel_size, channel_count) = match attachment.format {
        AttachmentFormat::Rgb8 => (1, 3),
        AttachmentFormat::Rgba8 => (1, 4),
        AttachmentFormat::R16 => (2, 1),
        AttachmentFormat::Rg16 => (2, 2),
    };

    let descriptor = TDF {
        pixel_size,
        channel_count,
        size: attachment.texture_size,
        mip_level_count: attachment.mip_level_count,
    };

    descriptor.save_file(path, node_image.as_bytes()).unwrap();
}

fn save_image_rs(path: &str, node_image: &DynamicImage, _attachment: &AttachmentConfig) {
    node_image.save(path).expect("Could not save node.");
}

fn save_dtm(path: &str, node_image: &DynamicImage, attachment: &AttachmentConfig) {
    let descriptor = DTM {
        pixel_size: 2,
        channel_count: match attachment.format {
            AttachmentFormat::Rgb8 => panic!("Can not save Rgb8 as DTM."),
            AttachmentFormat::Rgba8 => panic!("Can not save Rgba8 as DTM."),
            AttachmentFormat::R16 => 1,
            AttachmentFormat::Rg16 => 2,
        },
        width: node_image.width(),
        height: node_image.height(),
    };

    descriptor
        .encode_file(path, cast_slice(node_image.as_bytes()))
        .expect("Could not save node.");
}

fn save_qoi(path: &str, node_image: &DynamicImage, attachment: &AttachmentConfig) {
    let descriptor = Qoi {
        width: node_image.width(),
        height: node_image.height(),
        colors: match attachment.format {
            AttachmentFormat::Rgb8 => Colors::Rgb,
            AttachmentFormat::Rgba8 => Colors::Rgba,
            AttachmentFormat::R16 => panic!("Can not save R16 as QOI."),
            AttachmentFormat::Rg16 => panic!("Can not save Rg16 as QOI."),
        },
    };

    let bytes = descriptor
        .encode_alloc(cast_slice(node_image.as_bytes()))
        .unwrap();

    fs::write(path, bytes).expect("Could not save node.");
}
