use crate::{
    preprocess::{
        format_node_path, format_path, iterate_images, load_image, load_or_create_node,
        reset_directory, TileConfig, UVec2Utils,
    },
    skip_fail,
    terrain_data::{AttachmentConfig, AttachmentFormat},
    TerrainConfig,
};
use bevy::prelude::*;
use image::{
    imageops::{self, FilterType},
    DynamicImage, GenericImageView,
};
use itertools::iproduct;
use std::{fs, ops::Deref};

fn tile_to_node(
    node_image: &mut DynamicImage,
    tile_image: &DynamicImage,
    tile: &TileConfig,
    attachment: &AttachmentConfig,
    coord: UVec2,
) {
    let x =
        (tile.offset.x + attachment.border_size) as i64 - (coord.x * attachment.center_size) as i64;
    let y =
        (tile.offset.y + attachment.border_size) as i64 - (coord.y * attachment.center_size) as i64;

    match attachment.format {
        AttachmentFormat::Rgb8 => imageops::replace(
            node_image.as_mut_rgb8().unwrap(),
            tile_image.as_rgb8().unwrap(),
            x,
            y,
        ),
        AttachmentFormat::Rgba8 => imageops::replace(
            node_image.as_mut_rgba8().unwrap(),
            tile_image.as_rgba8().unwrap(),
            x,
            y,
        ),
        AttachmentFormat::R16 => imageops::replace(
            node_image.as_mut_luma16().unwrap(),
            tile_image.as_luma16().unwrap(),
            x,
            y,
        ),
        AttachmentFormat::Rg16 => imageops::replace(
            node_image.as_mut_luma_alpha16().unwrap(),
            tile_image.as_luma_alpha16().unwrap(),
            x,
            y,
        ),
    };
}

fn split_tile(directory: &str, tile: &TileConfig, attachment: &AttachmentConfig) {
    let tile_image = load_image(&tile.path).expect("Could not load tile.");

    // first and last node coordinate
    let first = tile.offset.div_floor(attachment.center_size);
    let last = (tile.offset + tile.size + attachment.border_size).div_ceil(attachment.center_size);

    for (x, y) in first.product(last) {
        let node_path = format_node_path(directory, tile.lod, x, y);

        let mut node_image = load_or_create_node(&node_path, attachment);

        tile_to_node(
            &mut node_image,
            &tile_image,
            tile,
            attachment,
            UVec2::new(x, y),
        );

        node_image.save(&node_path).expect("Could not save node.");
    }
}

fn split_tiles(
    directory: &str,
    tile: &TileConfig,
    attachment: &AttachmentConfig,
) -> (UVec2, UVec2) {
    let (offset, size) = if fs::metadata(&tile.path).unwrap().is_dir() {
        let mut min_pos = UVec2::splat(u32::MAX);
        let mut max_pos = UVec2::splat(u32::MIN);

        for (tile_name, tile_path) in iterate_images(&tile.path) {
            let mut parts = tile_name.split('_');
            parts.next();

            let coord = UVec2::new(
                parts.next().unwrap().parse::<u32>().unwrap(),
                parts.next().unwrap().parse::<u32>().unwrap(),
            );

            let tile = TileConfig {
                path: tile_path,
                offset: tile.offset + coord * tile.size,
                ..*tile
            };

            split_tile(directory, &tile, attachment);

            min_pos = min_pos.min(coord);
            max_pos = max_pos.max(coord);
        }

        let offset = tile.offset + min_pos * tile.size;
        let size = (1 + max_pos - min_pos) * tile.size;

        (offset, size)
    } else {
        split_tile(directory, tile, attachment);

        (tile.offset, UVec2::splat(tile.size))
    };

    let first = offset.div_floor(attachment.center_size);
    let last = (offset + size).div_ceil(attachment.center_size);

    (first, last)
}

fn down_sample(
    node_image: &mut DynamicImage,
    child_image: &DynamicImage,
    attachment: &AttachmentConfig,
    child_x: u32,
    child_y: u32,
) {
    let child_size = attachment.center_size >> 1;

    let x = (child_x * child_size + attachment.border_size) as i64;
    let y = (child_y * child_size + attachment.border_size) as i64;

    match attachment.format {
        AttachmentFormat::Rgb8 => {
            let child_image = child_image.as_rgb8().unwrap().view(
                attachment.border_size,
                attachment.border_size,
                attachment.center_size,
                attachment.center_size,
            );
            let child_image = imageops::resize(
                child_image.deref(),
                child_size,
                child_size,
                FilterType::Triangle,
            );

            imageops::replace(node_image.as_mut_rgb8().unwrap(), &child_image, x, y);
        }
        AttachmentFormat::Rgba8 => {
            let child_image = child_image.as_rgba8().unwrap().view(
                attachment.border_size,
                attachment.border_size,
                attachment.center_size,
                attachment.center_size,
            );
            let child_image = imageops::resize(
                child_image.deref(),
                child_size,
                child_size,
                FilterType::Triangle,
            );
            imageops::replace(node_image.as_mut_rgba8().unwrap(), &child_image, x, y);
        }
        AttachmentFormat::R16 => {
            let child_image = child_image.as_luma16().unwrap().view(
                attachment.border_size,
                attachment.border_size,
                attachment.center_size,
                attachment.center_size,
            );
            let child_image = imageops::resize(
                child_image.deref(),
                child_size,
                child_size,
                FilterType::Triangle,
            );
            imageops::replace(node_image.as_mut_luma16().unwrap(), &child_image, x, y);
        }
        AttachmentFormat::Rg16 => {
            let child_image = child_image.as_luma_alpha16().unwrap().view(
                attachment.border_size,
                attachment.border_size,
                attachment.center_size,
                attachment.center_size,
            );
            let child_image = imageops::resize(
                child_image.deref(),
                child_size,
                child_size,
                FilterType::Triangle,
            );
            imageops::replace(
                node_image.as_mut_luma_alpha16().unwrap(),
                &child_image,
                x,
                y,
            );
        }
    }
}

pub(crate) fn down_sample_layer(
    directory: &str,
    attachment: &AttachmentConfig,
    lod: u32,
    first: UVec2,
    last: UVec2,
) {
    for (x, y) in first.product(last) {
        let node_path = format_node_path(directory, lod, x, y);
        let mut node_image = load_or_create_node(&node_path, attachment);

        for (cx, cy) in iproduct!(0..2, 0..2) {
            let child_path = format_node_path(directory, lod - 1, (x << 1) + cx, (y << 1) + cy);
            let child_image = skip_fail!(load_image(&child_path));
            // Todo: if a child node is not available, we should fill the gap in the parent one
            // maybe this should not even be possible

            down_sample(&mut node_image, &child_image, attachment, cx, cy);
        }

        node_image.save(node_path).expect("Could not save node.");
    }
}

fn stitch(
    node_image: &mut DynamicImage,
    adjacent_image: &DynamicImage,
    attachment: &AttachmentConfig,
    direction: (i32, i32),
) {
    let w = match direction.0 {
        -1 => 0..attachment.border_size,
        0 => attachment.border_size..attachment.center_size + attachment.border_size,
        1 => attachment.center_size + attachment.border_size..attachment.texture_size(),
        _ => unreachable!(),
    };
    let h = match direction.1 {
        -1 => 0..attachment.border_size,
        0 => attachment.border_size..attachment.center_size + attachment.border_size,
        1 => attachment.center_size + attachment.border_size..attachment.texture_size(),
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
        1 => attachment.center_size + attachment.border_size..attachment.texture_size(),
        _ => unreachable!(),
    };
    let h = match direction.1 {
        -1 => 0..attachment.border_size,
        0 => attachment.border_size..attachment.center_size + attachment.border_size,
        1 => attachment.center_size + attachment.border_size..attachment.texture_size(),
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
        let mut node_image = skip_fail!(load_image(&node_path));

        for direction in iproduct!(-1..=1, -1..=1) {
            if direction == (0, 0) {
                continue;
            };

            let x = x as i32 + direction.0;
            let y = y as i32 + direction.1;

            let adjacent_path = format_node_path(directory, lod, x as u32, y as u32);

            if let Ok(adjacent_image) = load_image(&adjacent_path) {
                stitch(&mut node_image, &adjacent_image, attachment, direction);
            } else {
                extend(&mut node_image, attachment, direction);
            }
        }

        node_image.save(node_path).expect("Could not save node.");
    }
}

pub(crate) fn preprocess_attachment(
    config: &TerrainConfig,
    tile: &TileConfig,
    attachment: &AttachmentConfig,
) -> (UVec2, UVec2) {
    let directory = format_path(&config.path, &attachment.name);

    reset_directory(&directory);

    let (mut first, mut last) = split_tiles(&directory, tile, attachment);

    let output = (first, last);

    for lod in (tile.lod + 1)..config.lod_count {
        first = first.div_floor(2);
        last = last.div_ceil(2);

        down_sample_layer(&directory, attachment, lod, first, last);
        stitch_layer(&directory, attachment, lod, first, last);
    }

    output
}
