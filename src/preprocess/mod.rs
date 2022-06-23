use crate::quadtree::Node;
use image::{
    imageops::{self, FilterType},
    io::Reader,
    DynamicImage, GenericImage, GenericImageView, ImageBuffer, Luma, RgbImage, RgbaImage,
};
use itertools::iproduct;
use std::{fs, ops::Deref};

#[inline]
fn div_floor(x: u32, n: u32) -> u32 {
    x / n
}

#[inline]
fn div_ceil(x: u32, n: u32) -> u32 {
    (x + (n - 1)) / n
}

#[derive(Clone, Copy)]
pub enum ImageFormat {
    RGB,
    RGBA,
    LUMA16,
}

fn load_node(
    file_path: &str,
    texture_size: u32,
    border_size: u32,
    format: ImageFormat,
) -> DynamicImage {
    if let Ok(output) = image::open(file_path) {
        output
    } else {
        let size = texture_size + 2 * border_size;
        match format {
            ImageFormat::RGB => DynamicImage::from(RgbImage::new(size, size)),
            ImageFormat::RGBA => DynamicImage::from(RgbaImage::new(size, size)),
            ImageFormat::LUMA16 => DynamicImage::from(<ImageBuffer<Luma<u16>, _>>::new(size, size)),
        }
    }
}

fn overlay_node(
    bottom: &mut DynamicImage,
    top: &DynamicImage,
    x: i64,
    y: i64,
    format: ImageFormat,
) {
    match format {
        ImageFormat::RGB => {
            imageops::overlay(bottom.as_mut_rgb8().unwrap(), top.as_rgb8().unwrap(), x, y)
        }
        ImageFormat::RGBA => imageops::overlay(
            bottom.as_mut_rgba8().unwrap(),
            top.as_rgba8().unwrap(),
            x,
            y,
        ),
        ImageFormat::LUMA16 => imageops::overlay(
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
    texture_size: u32,
    border_size: u32,
    format: ImageFormat,
) {
    let child_size = texture_size >> 1;

    let x = child_x * child_size + border_size;
    let y = child_y * child_size + border_size;

    match format {
        ImageFormat::RGB => {
            let child_node = child_node.as_rgb8().unwrap();
            // crop the border away
            let child_node = child_node.view(border_size, border_size, texture_size, texture_size);
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
        ImageFormat::RGBA => {
            let child_node = child_node.as_rgba8().unwrap();
            // crop the border away
            let child_node = child_node.view(border_size, border_size, texture_size, texture_size);
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
        ImageFormat::LUMA16 => {
            let child_node = child_node.as_luma16().unwrap();
            // crop the border away
            let child_node = child_node.view(border_size, border_size, texture_size, texture_size);
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

pub fn stitch(
    destination: &mut DynamicImage,
    source: &DynamicImage,
    texture_size: u32,
    border_size: u32,
    format: ImageFormat,
    direction: (i32, i32),
) {
    let size = texture_size + 2 * border_size;
    let offset = texture_size + border_size;

    // positions to stitch
    let iter = match direction {
        (-1, 0) => iproduct!(0..border_size, 0..size)
            .map(|(b, i)| (b, i, texture_size + b, i))
            .collect::<Vec<_>>(),
        (1, 0) => iproduct!(0..border_size, 0..size)
            .map(|(b, i)| (offset + b, i, border_size + b, i))
            .collect::<Vec<_>>(),
        (0, -1) => iproduct!(0..border_size, 0..size)
            .map(|(b, i)| (i, b, i, texture_size + b))
            .collect::<Vec<_>>(),
        (0, 1) => iproduct!(0..border_size, 0..size)
            .map(|(b, i)| (i, offset + b, i, border_size + b))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    match format {
        ImageFormat::RGB => {
            let destination = destination.as_mut_rgb8().unwrap();
            let source = source.as_rgb8().unwrap();

            for (x1, y1, x2, y2) in iter {
                destination.put_pixel(x1, y1, *source.get_pixel(x2, y2));
            }
        }
        ImageFormat::RGBA => {
            let destination = destination.as_mut_rgba8().unwrap();
            let source = source.as_rgba8().unwrap();

            for (x1, y1, x2, y2) in iter {
                destination.put_pixel(x1, y1, *source.get_pixel(x2, y2));
            }
        }
        ImageFormat::LUMA16 => {
            let destination = destination.as_mut_luma16().unwrap();
            let source = source.as_luma16().unwrap();

            for (x1, y1, x2, y2) in iter {
                destination.put_pixel(x1, y1, *source.get_pixel(x2, y2));
            }
        }
    }
}

pub fn split_tile(
    input_file_path: &str,
    output_directory: &str,
    offset: (u32, u32),
    lod: u32,
    tile_size: u32,
    texture_size: u32,
    border_size: u32,
    format: ImageFormat,
) {
    let mut reader = Reader::open(input_file_path).unwrap();
    reader.no_limits();
    let tile = reader.decode().unwrap();

    // first and last chunk coordinate
    let first = (offset.0 / texture_size, offset.1 / texture_size);
    let last = (
        div_ceil(offset.0 + tile_size + 2 * border_size, texture_size),
        div_ceil(offset.1 + tile_size + 2 * border_size, texture_size),
    );

    for (x, y) in iproduct!(first.0..last.0, first.1..last.1) {
        let node_id = Node::id(lod, x, y);
        let file_path = format!("{output_directory}/{node_id}.png");

        let mut node = load_node(&file_path, texture_size, border_size, format);

        let dx = (offset.0 + border_size) as i64 - (x * texture_size) as i64;
        let dy = (offset.1 + border_size) as i64 - (y * texture_size) as i64;

        overlay_node(&mut node, &tile, dx, dy, format);

        node.save(&file_path).expect("Could not save file.");
    }
}

pub fn down_sample_nodes(
    directory: &str,
    first: (u32, u32),
    last: (u32, u32),
    lod: u32,
    texture_size: u32,
    border_size: u32,
    format: ImageFormat,
) {
    for (x, y) in iproduct!(first.0..last.0, first.1..last.1) {
        let node_id = Node::id(lod, x, y);
        let file_path = format!("{directory}/{node_id}.png");

        let mut node = load_node(&file_path, texture_size, border_size, format);

        let child_origin = (x << 1, y << 1);
        let child_lod = lod - 1;

        for (cx, cy) in iproduct!(0..2, 0..2) {
            let child_id = Node::id(child_lod, child_origin.0 + cx, child_origin.1 + cy);
            let child_path = format!("{directory}/{child_id}.png");

            let child_node = load_node(&child_path, texture_size, border_size, format);

            down_sample_overlay(
                &mut node,
                &child_node,
                cx,
                cy,
                texture_size,
                border_size,
                format,
            );
        }

        node.save(file_path).expect("Could not save file.");
    }
}

pub fn stitch_nodes(
    directory: &str,
    first: (u32, u32),
    last: (u32, u32),
    lod: u32,
    texture_size: u32,
    border_size: u32,
    format: ImageFormat,
) {
    for (x, y) in iproduct!(first.0..last.0, first.1..last.1) {
        let node_id = Node::id(lod, x, y);
        let file_path = format!("{directory}/{node_id}.png");

        let mut node = load_node(&file_path, texture_size, border_size, format);

        // Todo: should include corners as well
        for direction in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
            let x = x as i32 + direction.0;
            let y = y as i32 + direction.1;

            if x < 0 || y < 0 {
                continue;
            };

            let adjacent_id = Node::id(lod, x as u32, y as u32);
            let adjacent_path = format!("{directory}/{adjacent_id}.png");

            let adjacent_node = load_node(&adjacent_path, texture_size, border_size, format);

            stitch(
                &mut node,
                &adjacent_node,
                texture_size,
                border_size,
                format,
                direction,
            );
        }

        node.save(file_path).expect("Could not save file.");
    }
}

pub fn preprocess_tiles(
    input_path: &str,
    output_directory: &str,
    base_lod: u32,
    lod_count: u32,
    offset: (u32, u32),
    tile_size: u32,
    texture_size: u32,
    border_size: u32,
    format: ImageFormat,
) {
    let _ = fs::remove_dir_all(output_directory);
    fs::create_dir_all(output_directory).unwrap();

    let (offset, size) = if fs::metadata(input_path)
        .expect("Could not find the input path.")
        .is_dir()
    {
        let mut min_pos = (u32::MAX, u32::MAX);
        let mut max_pos = (u32::MIN, u32::MIN);

        for file_path in fs::read_dir(input_path)
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

            let x = parts.next().unwrap().parse::<u32>().unwrap();
            let y = parts.next().unwrap().parse::<u32>().unwrap();

            min_pos = (min_pos.0.min(x), min_pos.1.min(y));
            max_pos = (max_pos.0.max(x), max_pos.1.max(y));

            split_tile(
                file_path.to_str().unwrap(),
                output_directory,
                (x * tile_size + offset.0, y * tile_size + offset.1),
                base_lod,
                tile_size,
                texture_size,
                border_size,
                format,
            );
        }

        let offset = (
            offset.0 + min_pos.0 * tile_size,
            offset.1 + min_pos.1 * tile_size,
        );
        let size = (
            (max_pos.0 - min_pos.0 + 1) * tile_size,
            (max_pos.1 - min_pos.1 + 1) * tile_size,
        );

        (offset, size)
    } else {
        split_tile(
            input_path,
            output_directory,
            offset,
            base_lod,
            tile_size,
            texture_size,
            border_size,
            format,
        );
        (offset, (tile_size, tile_size))
    };

    let mut first = (
        div_floor(offset.0, texture_size),
        div_floor(offset.1, texture_size),
    );
    let mut last = (
        div_ceil(offset.0 + size.0 + 2 * border_size, texture_size),
        div_ceil(offset.1 + size.1 + 2 * border_size, texture_size),
    );

    for lod in 1..lod_count {
        first = (div_floor(first.0, 2), div_floor(first.1, 2));
        last = (div_ceil(last.0, 2), div_ceil(last.1, 2));

        down_sample_nodes(
            output_directory,
            first,
            last,
            lod,
            texture_size,
            border_size,
            format,
        );

        stitch_nodes(
            output_directory,
            first,
            last,
            lod,
            texture_size,
            border_size,
            format,
        );
    }
}
