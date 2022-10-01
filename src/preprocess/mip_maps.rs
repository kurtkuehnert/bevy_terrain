use crate::loader::tdf::TDF;
use crate::preprocess::down_sample::imageops_linear;
use crate::preprocess::file_io::load_image;
use crate::preprocess::{R16Image, Rg16Image, Rgb8Image, Rgba8Image};
use crate::terrain_data::{AttachmentConfig, AttachmentFormat};
use image::DynamicImage;
use walkdir::{DirEntry, WalkDir};

fn iterate_dir(directory: &str) -> Vec<DirEntry> {
    let walker = WalkDir::new(directory).into_iter();
    walker
        .filter_entry(|entry| {
            if entry.path().is_dir() {
                return true;
            }
            if let Some(extension) = entry.path().extension() {
                if (extension == "dtm" || extension == "qoi" || extension == "tdf")
                    && !entry
                        .file_name()
                        .to_str()
                        .map(|s| s.starts_with("."))
                        .unwrap_or(false)
                {
                    return true;
                }
            }

            false
        })
        .filter_map(|entry| {
            let entry = entry.unwrap();

            if entry.path().is_dir() {
                None
            } else {
                Some(entry)
            }
        })
        .collect::<Vec<_>>()
}

fn is_power_of_two(x: u32) -> bool {
    (x & (x - 1)) == 0
}

pub(crate) fn generate_mip_maps(directory: &str, attachment: &AttachmentConfig) {
    if attachment.mip_level_count == 1 {
        return;
    }

    for entry in iterate_dir(directory) {
        let mut size = attachment.texture_size;
        let mut images = vec![DynamicImage::new_luma8(0, 0); attachment.mip_level_count as usize];
        images[0] = load_image(entry.path().to_str().unwrap(), attachment.file_format).unwrap();

        for i in 1..attachment.mip_level_count as usize {
            assert!(is_power_of_two(size));

            size /= 2;
            let child_image = &images[i - 1];

            let mut parent_image = match attachment.format {
                AttachmentFormat::Rgb8 => DynamicImage::from(Rgb8Image::new(size, size)),
                AttachmentFormat::Rgba8 => DynamicImage::from(Rgba8Image::new(size, size)),
                AttachmentFormat::R16 => DynamicImage::from(R16Image::new(size, size)),
                AttachmentFormat::Rg16 => DynamicImage::from(Rg16Image::new(size, size)),
            };

            linear(&mut parent_image, child_image, attachment, size);

            images[i] = parent_image;
        }

        let mut data = Vec::new();

        for image in images {
            data.extend_from_slice(image.as_bytes());
        }

        let (pixel_size, channel_count) = match attachment.format {
            AttachmentFormat::Rgb8 => (1, 3),
            AttachmentFormat::Rgba8 => (1, 4),
            AttachmentFormat::R16 => (2, 1),
            AttachmentFormat::Rg16 => (2, 2),
        };

        let descriptor = TDF {
            pixel_size,
            channel_count,
            width: attachment.texture_size,
            height: attachment.texture_size,
            mip_count: attachment.mip_level_count,
        };

        descriptor.save_file(entry.path(), &data);
    }
}

pub(crate) fn linear(
    parent_image: &mut DynamicImage,
    child_image: &DynamicImage,
    attachment: &AttachmentConfig,
    child_size: u32,
) {
    match attachment.format {
        AttachmentFormat::Rgb8 => {
            imageops_linear(
                parent_image.as_mut_rgb8().unwrap(),
                child_image.as_rgb8().unwrap(),
                child_size,
                0,
                0,
                0,
            );
        }
        AttachmentFormat::Rgba8 => {
            imageops_linear(
                parent_image.as_mut_rgba8().unwrap(),
                child_image.as_rgba8().unwrap(),
                child_size,
                0,
                0,
                0,
            );
        }
        AttachmentFormat::R16 => {
            imageops_linear(
                parent_image.as_mut_luma16().unwrap(),
                child_image.as_luma16().unwrap(),
                child_size,
                0,
                0,
                0,
            );
        }
        AttachmentFormat::Rg16 => {
            imageops_linear(
                parent_image.as_mut_luma_alpha16().unwrap(),
                child_image.as_luma_alpha16().unwrap(),
                child_size,
                0,
                0,
                0,
            );
        }
    }
}
