use crate::preprocess::{R16Image, Rg16Image, Rgb8Image, Rgba8Image};
use crate::terrain_data::{calc_node_id, AttachmentConfig, AttachmentFormat, FileFormat};
use bytemuck::cast_slice;
use dtm::DTM;
use image::{io::Reader, DynamicImage, ImageResult};
use rapid_qoi::{Colors, Qoi};
use std::{
    fs::{self, DirEntry, ReadDir},
    iter::Map,
};

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

pub(crate) fn reset_directory(directory: &str) {
    let _ = fs::remove_dir_all(directory);
    fs::create_dir_all(directory).unwrap();
}

pub(crate) fn format_directory(path: &str, name: &str) -> String {
    format!("assets/{path}/data/{name}")
}

pub(crate) fn format_node_path(
    directory: &str,
    attachment: &AttachmentConfig,
    lod: u32,
    x: u32,
    y: u32,
) -> String {
    let node_id = calc_node_id(lod, x, y);

    format!(
        "{directory}/{node_id}.{}",
        attachment.file_format.extension()
    )
}

pub(crate) fn load_image(file_path: &str) -> ImageResult<DynamicImage> {
    let mut reader = Reader::open(file_path)?;
    reader.no_limits();
    reader.decode()
}

pub(crate) fn load_node(node_path: &str, attachment: &AttachmentConfig) -> Option<DynamicImage> {
    match attachment.file_format {
        FileFormat::BIN => load_bin(node_path, attachment),
        FileFormat::PNG => load_png(node_path, attachment),
        FileFormat::QOI => load_qoi(node_path, attachment),
        FileFormat::DTM => load_dtm(node_path, attachment),
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

pub(crate) fn save_node(node_path: &str, node_image: &DynamicImage, attachment: &AttachmentConfig) {
    match attachment.file_format {
        FileFormat::BIN => save_bin(node_path, node_image, attachment),
        FileFormat::PNG => save_png(node_path, node_image, attachment),
        FileFormat::QOI => save_qoi(node_path, node_image, attachment),
        FileFormat::DTM => save_dtm(node_path, node_image, attachment),
    }
}

fn load_bin(node_path: &str, attachment: &AttachmentConfig) -> Option<DynamicImage> {
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

fn load_png(node_path: &str, _attachment: &AttachmentConfig) -> Option<DynamicImage> {
    image::open(node_path).ok()
}

fn load_dtm(node_path: &str, attachment: &AttachmentConfig) -> Option<DynamicImage> {
    let size = attachment.texture_size();

    let (descriptor, pixels) = DTM::decode_file(node_path).ok()?;

    match descriptor.channel_count {
        1 => {
            let image = R16Image::from_raw(size, size, pixels).unwrap();
            Some(DynamicImage::from(image))
        }
        2 => {
            let image = Rg16Image::from_raw(size, size, pixels).unwrap();
            Some(DynamicImage::from(image))
        }
        _ => None,
    }
}

fn load_qoi(node_path: &str, attachment: &AttachmentConfig) -> Option<DynamicImage> {
    let size = attachment.texture_size();

    let bytes = fs::read(node_path).ok()?;
    let (descriptor, pixels) = Qoi::decode_alloc(&bytes).unwrap();

    match descriptor.colors {
        Colors::Rgb => {
            let image = Rgb8Image::from_raw(size, size, pixels).unwrap();
            Some(DynamicImage::from(image))
        }
        Colors::Rgba => {
            let image = Rgba8Image::from_raw(size, size, pixels).unwrap();
            Some(DynamicImage::from(image))
        }
        _ => None,
    }
}

fn save_bin(node_path: &str, node_image: &DynamicImage, _attachment: &AttachmentConfig) {
    fs::write(node_path, node_image.as_bytes()).expect("Could not save node.");
}

fn save_png(node_path: &str, node_image: &DynamicImage, _attachment: &AttachmentConfig) {
    node_image.save(node_path).expect("Could not save node.");
}

fn save_dtm(node_path: &str, node_image: &DynamicImage, attachment: &AttachmentConfig) {
    let descriptor = DTM {
        pixel_size: 2,
        channel_count: match attachment.format {
            AttachmentFormat::Rgb8 => panic!("Can not save Rgb8 as DTM."),
            AttachmentFormat::Rgba8 => panic!("Can not save Rgba8 as DTM."),
            AttachmentFormat::R16 => 1,
            AttachmentFormat::Rg16 => 2,
        },
        width: node_image.width() as usize,
        height: node_image.height() as usize,
    };

    descriptor
        .encode_file(node_path, cast_slice(node_image.as_bytes()))
        .expect("Could not save node.");
}

fn save_qoi(node_path: &str, node_image: &DynamicImage, attachment: &AttachmentConfig) {
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

    fs::write(node_path, &bytes).expect("Could not save node.");
}
