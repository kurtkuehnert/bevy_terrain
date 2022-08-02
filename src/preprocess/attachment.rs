use crate::{
    data_structures::{AttachmentConfig, AttachmentFormat},
    preprocess::{format_path, load_node, node_path, read_image, TileConfig, UVec2Utils},
    TerrainConfig,
};
use bevy::prelude::UVec2;
use image::{
    imageops::{self, FilterType},
    DynamicImage, GenericImage, GenericImageView,
};
use itertools::iproduct;
use std::{fs, ops::Deref};

fn overlay_node(
    bottom: &mut DynamicImage,
    top: &DynamicImage,
    x: i64,
    y: i64,
    format: AttachmentFormat,
) {
    match format {
        AttachmentFormat::RGB => {
            imageops::overlay(bottom.as_mut_rgb8().unwrap(), top.as_rgb8().unwrap(), x, y)
        }
        AttachmentFormat::RGBA => imageops::overlay(
            bottom.as_mut_rgba8().unwrap(),
            top.as_rgba8().unwrap(),
            x,
            y,
        ),
        AttachmentFormat::LUMA16 => imageops::overlay(
            bottom.as_mut_luma16().unwrap(),
            top.as_luma16().unwrap(),
            x,
            y,
        ),
    };
}

fn down_sample_overlay(
    node: &mut DynamicImage,
    child_node: &DynamicImage,
    child_x: u32,
    child_y: u32,
    center_size: u32,
    border_size: u32,
    format: AttachmentFormat,
) {
    let child_size = center_size >> 1;

    let x = child_x * child_size + border_size;
    let y = child_y * child_size + border_size;

    match format {
        AttachmentFormat::RGB => {
            let child_node = child_node.as_rgb8().unwrap();
            // crop the border away
            let child_node = child_node.view(border_size, border_size, center_size, center_size);
            // down sample to half quarter the resolution
            let child_node = imageops::resize(
                child_node.deref(),
                child_size,
                child_size,
                FilterType::Triangle,
            );
            node.as_mut_rgb8()
                .unwrap()
                .copy_from(&child_node, x, y)
                .unwrap();
        }
        AttachmentFormat::RGBA => {
            let child_node = child_node.as_rgba8().unwrap();
            // crop the border away
            let child_node = child_node.view(border_size, border_size, center_size, center_size);
            // down sample to half quarter the resolution
            let child_node = imageops::resize(
                child_node.deref(),
                child_size,
                child_size,
                FilterType::Triangle,
            );
            node.as_mut_rgba8()
                .unwrap()
                .copy_from(&child_node, x, y)
                .unwrap();
        }
        AttachmentFormat::LUMA16 => {
            let child_node = child_node.as_luma16().unwrap();
            // crop the border away
            let child_node = child_node.view(border_size, border_size, center_size, center_size);
            // down sample to half quarter the resolution
            let child_node = imageops::resize(
                child_node.deref(),
                child_size,
                child_size,
                FilterType::Triangle,
            );
            node.as_mut_luma16()
                .unwrap()
                .copy_from(&child_node, x, y)
                .unwrap();
        }
    }
}

fn stitch(
    destination: &mut DynamicImage,
    source: &DynamicImage,
    center_size: u32,
    border_size: u32,
    format: AttachmentFormat,
    direction: (i32, i32),
) {
    let size = center_size + 2 * border_size;
    let offset = center_size + border_size;

    // positions to stitch
    let iter = match direction {
        (-1, 0) => iproduct!(0..border_size, 0..size)
            .map(|(b, i)| (b, i, center_size + b, i))
            .collect::<Vec<_>>(),
        (1, 0) => iproduct!(0..border_size, 0..size)
            .map(|(b, i)| (offset + b, i, border_size + b, i))
            .collect::<Vec<_>>(),
        (0, -1) => iproduct!(0..border_size, 0..size)
            .map(|(b, i)| (i, b, i, center_size + b))
            .collect::<Vec<_>>(),
        (0, 1) => iproduct!(0..border_size, 0..size)
            .map(|(b, i)| (i, offset + b, i, border_size + b))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    match format {
        AttachmentFormat::RGB => {
            let destination = destination.as_mut_rgb8().unwrap();
            let source = source.as_rgb8().unwrap();

            for (x1, y1, x2, y2) in iter {
                destination.put_pixel(x1, y1, *source.get_pixel(x2, y2));
            }
        }
        AttachmentFormat::RGBA => {
            let destination = destination.as_mut_rgba8().unwrap();
            let source = source.as_rgba8().unwrap();

            for (x1, y1, x2, y2) in iter {
                destination.put_pixel(x1, y1, *source.get_pixel(x2, y2));
            }
        }
        AttachmentFormat::LUMA16 => {
            let destination = destination.as_mut_luma16().unwrap();
            let source = source.as_luma16().unwrap();

            for (x1, y1, x2, y2) in iter {
                destination.put_pixel(x1, y1, *source.get_pixel(x2, y2));
            }
        }
    }
}

fn split_tile(
    source_file_path: &str,
    output_directory: &str,
    offset: UVec2,
    lod: u32,
    tile_size: u32,
    center_size: u32,
    border_size: u32,
    format: AttachmentFormat,
) {
    let tile = read_image(source_file_path);

    // first and last chunk coordinate
    let first = offset.div_floor(center_size);
    let last = (offset + tile_size + border_size).div_ceil(center_size);

    for (x, y) in first.product(last) {
        let file_path = node_path(output_directory, lod, x, y);

        let mut node = load_node(&file_path, center_size, border_size, format);

        let dx = (offset.x + border_size) as i64 - (x * center_size) as i64;
        let dy = (offset.y + border_size) as i64 - (y * center_size) as i64;

        overlay_node(&mut node, &tile, dx, dy, format);

        node.save(&file_path).expect("Could not save file.");
    }
}

pub(crate) fn down_sample_nodes(
    directory: &str,
    first: UVec2,
    last: UVec2,
    lod: u32,
    center_size: u32,
    border_size: u32,
    format: AttachmentFormat,
) {
    for (x, y) in first.product(last) {
        let file_path = node_path(directory, lod, x, y);

        let mut node = load_node(&file_path, center_size, border_size, format);

        for (cx, cy) in iproduct!(0..2, 0..2) {
            let child_path = node_path(directory, lod - 1, (x << 1) + cx, (y << 1) + cy);

            let child_node = load_node(&child_path, center_size, border_size, format);

            down_sample_overlay(
                &mut node,
                &child_node,
                cx,
                cy,
                center_size,
                border_size,
                format,
            );
        }

        node.save(file_path).expect("Could not save file.");
    }
}

fn stitch_nodes(
    directory: &str,
    first: UVec2,
    last: UVec2,
    lod: u32,
    center_size: u32,
    border_size: u32,
    format: AttachmentFormat,
) {
    for (x, y) in first.product(last) {
        let file_path = node_path(directory, lod, x, y);

        let mut node = load_node(&file_path, center_size, border_size, format);

        // Todo: should include corners as well
        for direction in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
            let x = x as i32 + direction.0;
            let y = y as i32 + direction.1;

            if x < 0 || y < 0 {
                continue;
            };

            let adjacent_path = node_path(directory, lod, x as u32, y as u32);

            let adjacent_node = load_node(&adjacent_path, center_size, border_size, format);

            stitch(
                &mut node,
                &adjacent_node,
                center_size,
                border_size,
                format,
                direction,
            );
        }

        node.save(file_path).expect("Could not save file.");
    }
}

fn preprocess_tiles(
    source_path: &str,
    output_directory: &str,
    base_lod: u32,
    offset: UVec2,
    tile_size: u32,
    center_size: u32,
    border_size: u32,
    format: AttachmentFormat,
) -> (UVec2, UVec2) {
    let (offset, size) = if fs::metadata(source_path)
        .expect("Could not find the source path.")
        .is_dir()
    {
        let mut min_pos = UVec2::splat(u32::MAX);
        let mut max_pos = UVec2::splat(u32::MIN);

        for file_path in fs::read_dir(source_path)
            .unwrap()
            .map(|path| path.unwrap().path())
        {
            let file_name = file_path
                .with_extension("")
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            let mut parts = file_name.split('_');
            parts.next();

            let coord = UVec2::new(
                parts.next().unwrap().parse::<u32>().unwrap(),
                parts.next().unwrap().parse::<u32>().unwrap(),
            );

            min_pos = min_pos.min(coord);
            max_pos = max_pos.max(coord);

            split_tile(
                file_path.to_str().unwrap(),
                output_directory,
                coord * tile_size + offset,
                base_lod,
                tile_size,
                center_size,
                border_size,
                format,
            );
        }

        let offset = offset + min_pos * tile_size;
        let size = (1 + max_pos - min_pos) * tile_size;

        (offset, size)
    } else {
        split_tile(
            source_path,
            output_directory,
            offset,
            base_lod,
            tile_size,
            center_size,
            border_size,
            format,
        );

        (offset, UVec2::splat(tile_size))
    };

    let first = offset.div_floor(center_size);
    let last = (offset + size).div_ceil(center_size);

    (first, last)
}

pub(crate) fn preprocess_attachment(
    config: &TerrainConfig,
    tile: &TileConfig,
    attachment: &AttachmentConfig,
) -> (UVec2, UVec2) {
    let output_directory = format_path(&config.path, attachment.name);

    let _ = fs::remove_dir_all(&output_directory);
    fs::create_dir_all(&output_directory).unwrap();

    let (first, last) = preprocess_tiles(
        tile.path,
        &output_directory,
        tile.lod,
        tile.offset,
        tile.size,
        attachment.center_size,
        attachment.border_size,
        attachment.format,
    );

    let mut tmp_first = first;
    let mut tmp_last = last;

    for lod in (tile.lod + 1)..config.lod_count {
        tmp_first = tmp_first.div_floor(2);
        tmp_last = tmp_last.div_ceil(2);

        down_sample_nodes(
            &output_directory,
            tmp_first,
            tmp_last,
            lod,
            attachment.center_size,
            attachment.border_size,
            attachment.format,
        );

        stitch_nodes(
            &output_directory,
            tmp_first,
            tmp_last,
            lod,
            attachment.center_size,
            attachment.border_size,
            attachment.format,
        );
    }

    (first, last)
}
