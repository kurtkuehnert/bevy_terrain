use crate::quadtree::Node;
use image::{
    imageops::{self, FilterType},
    DynamicImage, GenericImageView, ImageBuffer, Luma, RgbImage, RgbaImage,
};
use itertools::iproduct;
use std::{fs, ops::Deref};

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

    let x = (child_x * child_size + border_size) as i64;
    let y = (child_y * child_size + border_size) as i64;

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
            imageops::overlay(node.as_mut_rgb8().unwrap(), &child_node, x, y);
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
            imageops::overlay(node.as_mut_rgba8().unwrap(), &child_node, x, y);
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
            imageops::overlay(node.as_mut_luma16().unwrap(), &child_node, x, y);
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
    let tile = image::open(input_file_path).unwrap();

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

pub fn downscale_nodes(
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

pub fn downscale_all(
    directory: &str,
    offset: (u32, u32),
    size: (u32, u32),
    lod_count: u32,
    texture_size: u32,
    border_size: u32,
    format: ImageFormat,
) {
    let mut first = (offset.0 / texture_size, offset.1 / texture_size);
    let mut last = (
        div_ceil(offset.0 + size.0 + 2 * border_size, texture_size),
        div_ceil(offset.1 + size.1 + 2 * border_size, texture_size),
    );

    for lod in 1..lod_count {
        first = (first.0 / 2, first.1 / 2);
        last = (div_ceil(last.0, 2), div_ceil(last.1, 2));

        downscale_nodes(
            directory,
            first,
            last,
            lod,
            texture_size,
            border_size,
            format,
        );
    }
}

pub fn preprocess_tiles(
    input_directory: &str,
    output_directory: &str,
    base_lod: u32,
    lod_count: u32,
    offset: (u32, u32),
    tile_size: u32,
    texture_size: u32,
    border_size: u32,
    format: ImageFormat,
) {
    fs::remove_dir_all(output_directory).unwrap();
    fs::create_dir(output_directory).unwrap();

    let paths = fs::read_dir(input_directory).expect("Could not find the input directory.");

    let mut min_pos = (u32::MAX, u32::MAX);
    let mut max_pos = (u32::MIN, u32::MIN);

    for path in paths {
        let file_path = path.unwrap().path();
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
        offset.0 + (max_pos.0 + 1) * tile_size - offset.0,
        offset.1 + (max_pos.1 + 1) * tile_size - offset.1,
    );

    downscale_all(
        output_directory,
        offset,
        size,
        lod_count,
        texture_size,
        border_size,
        format,
    );
}
