use crate::{
    preprocess::{
        file_io::{format_node_path, load_image, save_image},
        UVec2Utils,
    },
    skip_none,
    terrain_data::{AttachmentConfig, AttachmentFormat},
};
use bevy::prelude::*;
use image::DynamicImage;
use itertools::iproduct;

fn stitch(
    node_image: &mut DynamicImage,
    adjacent_image: &DynamicImage,
    attachment: &AttachmentConfig,
    direction: (i32, i32),
) {
    let w = match direction.0 {
        -1 => 0..attachment.border_size,
        0 => attachment.border_size..attachment.center_size + attachment.border_size,
        1 => attachment.center_size + attachment.border_size..attachment.texture_size,
        _ => unreachable!(),
    };
    let h = match direction.1 {
        -1 => 0..attachment.border_size,
        0 => attachment.border_size..attachment.center_size + attachment.border_size,
        1 => attachment.center_size + attachment.border_size..attachment.texture_size,
        _ => unreachable!(),
    };

    let iter = iproduct!(w, h).map(|(x1, y1)| {
        let x2 = (x1 as i32 - direction.0 * attachment.center_size as i32) as u32;
        let y2 = (y1 as i32 - direction.1 * attachment.center_size as i32) as u32;

        (x1, y1, x2, y2)
    });

    match attachment.format {
        AttachmentFormat::Rgb8 => {
            let node_image = node_image.as_mut_rgb8().unwrap();
            let adjacent_image = adjacent_image.as_rgb8().unwrap();

            for (x1, y1, x2, y2) in iter {
                node_image.put_pixel(x1, y1, *adjacent_image.get_pixel(x2, y2));
            }
        }
        AttachmentFormat::Rgba8 => {
            let node_image = node_image.as_mut_rgba8().unwrap();
            let adjacent_image = adjacent_image.as_rgba8().unwrap();

            for (x1, y1, x2, y2) in iter {
                node_image.put_pixel(x1, y1, *adjacent_image.get_pixel(x2, y2));
            }
        }
        AttachmentFormat::R16 => {
            let node_image = node_image.as_mut_luma16().unwrap();
            let adjacent_image = adjacent_image.as_luma16().unwrap();

            for (x1, y1, x2, y2) in iter {
                node_image.put_pixel(x1, y1, *adjacent_image.get_pixel(x2, y2));
            }
        }
        AttachmentFormat::Rg16 => {
            let node_image = node_image.as_mut_luma_alpha16().unwrap();
            let adjacent_image = adjacent_image.as_luma_alpha16().unwrap();

            for (x1, y1, x2, y2) in iter {
                node_image.put_pixel(x1, y1, *adjacent_image.get_pixel(x2, y2));
            }
        }
    }
}

fn extend(node_image: &mut DynamicImage, attachment: &AttachmentConfig, direction: (i32, i32)) {
    let w = match direction.0 {
        -1 => 0..attachment.border_size,
        0 => attachment.border_size..attachment.center_size + attachment.border_size,
        1 => attachment.center_size + attachment.border_size..attachment.texture_size,
        _ => unreachable!(),
    };
    let h = match direction.1 {
        -1 => 0..attachment.border_size,
        0 => attachment.border_size..attachment.center_size + attachment.border_size,
        1 => attachment.center_size + attachment.border_size..attachment.texture_size,
        _ => unreachable!(),
    };

    let iter = iproduct!(w, h).map(|(x1, y1)| {
        let x2 = (x1 as i32 - direction.0 * attachment.border_size as i32) as u32;
        let y2 = (y1 as i32 - direction.1 * attachment.border_size as i32) as u32;

        (x1, y1, x2, y2)
    });

    match attachment.format {
        AttachmentFormat::Rgb8 => {
            let node_image = node_image.as_mut_rgb8().unwrap();

            for (x1, y1, x2, y2) in iter {
                node_image.put_pixel(x1, y1, *node_image.get_pixel(x2, y2));
            }
        }
        AttachmentFormat::Rgba8 => {
            let node_image = node_image.as_mut_rgba8().unwrap();

            for (x1, y1, x2, y2) in iter {
                node_image.put_pixel(x1, y1, *node_image.get_pixel(x2, y2));
            }
        }
        AttachmentFormat::R16 => {
            let node_image = node_image.as_mut_luma16().unwrap();

            for (x1, y1, x2, y2) in iter {
                node_image.put_pixel(x1, y1, *node_image.get_pixel(x2, y2));
            }
        }
        AttachmentFormat::Rg16 => {
            let node_image = node_image.as_mut_luma_alpha16().unwrap();

            for (x1, y1, x2, y2) in iter {
                node_image.put_pixel(x1, y1, *node_image.get_pixel(x2, y2));
            }
        }
    }
}

pub(crate) fn stitch_layer(
    directory: &str,
    attachment: &AttachmentConfig,
    lod: u32,
    first: UVec2,
    last: UVec2,
) {
    if attachment.border_size == 0 {
        return;
    }

    for (x, y) in first.product(last) {
        let node_path = format_node_path(directory, lod, x, y);
        let mut node_image = skip_none!(load_image(&node_path, attachment.file_format));

        for direction in iproduct!(-1..=1, -1..=1) {
            if direction == (0, 0) {
                continue;
            };

            let x = x as i32 + direction.0;
            let y = y as i32 + direction.1;

            let adjacent_path = format_node_path(directory, lod, x as u32, y as u32);

            if let Some(adjacent_image) = load_image(&adjacent_path, attachment.file_format) {
                stitch(&mut node_image, &adjacent_image, attachment, direction);
            } else {
                extend(&mut node_image, attachment, direction);
            }
        }

        save_image(&node_path, &node_image, attachment);
    }
}
