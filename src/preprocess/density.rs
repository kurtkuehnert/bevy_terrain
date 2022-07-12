use crate::preprocess::{div_ceil, div_floor, down_sample_nodes, load_node, ImageFormat};
use crate::quadtree::Node;
use crate::Vec3;
use image::{DynamicImage, ImageBuffer, Luma};
use itertools::iproduct;

fn height_to_density(
    height_node: &DynamicImage,
    texture_size: u32,
    border_size: u32,
    height: f32,
) -> DynamicImage {
    let height_node = height_node.as_luma16().unwrap();

    let density_node = ImageBuffer::from_fn(texture_size, texture_size, |x, y| {
        let left = height_node
            .get_pixel(x + border_size - 1, y + border_size)
            .0[0] as f32
            / u16::MAX as f32;
        let up = height_node
            .get_pixel(x + border_size, y + border_size - 1)
            .0[0] as f32
            / u16::MAX as f32;
        let right = height_node
            .get_pixel(x + border_size + 1, y + border_size)
            .0[0] as f32
            / u16::MAX as f32;
        let down = height_node
            .get_pixel(x + border_size, y + border_size + 1)
            .0[0] as f32
            / u16::MAX as f32;

        let normal = Vec3::new(right - left, 2.0 / height, down - up).normalize();
        let slope = 1.0 - normal.dot(Vec3::new(0.0, 1.0, 0.0));
        let slope = (slope * u16::MAX as f32) as u16;

        Luma([slope])
    });

    DynamicImage::from(density_node)
}

pub fn density_chunks(
    height_directory: &str,
    density_directory: &str,
    first: (u32, u32),
    last: (u32, u32),
    texture_size: u32,
    border_size: u32,
    height: f32,
) {
    for (x, y) in iproduct!(first.0..last.0, first.1..last.1) {
        let node_id = Node::id(0, x, y);
        let density_file_path = format!("{density_directory}/{node_id}.png");
        let height_file_path = format!("{height_directory}/{node_id}.png");

        let height_node = load_node(
            &height_file_path,
            texture_size,
            border_size,
            ImageFormat::LUMA16,
        );

        let density_node = height_to_density(&height_node, texture_size, border_size, height);

        density_node
            .save(density_file_path)
            .expect("Could not save file.");
    }
}

pub fn preprocess_density(
    height_directory: &str,
    density_directory: &str,
    lod_count: u32,
    first: (u32, u32),
    last: (u32, u32),
    texture_size: u32,
    border_size: u32,
    height: f32,
) {
    density_chunks(
        height_directory,
        density_directory,
        first,
        last,
        texture_size,
        border_size,
        height,
    );

    let mut first = first;
    let mut last = last;

    for lod in 1..lod_count {
        first = (div_floor(first.0, 2), div_floor(first.1, 2));
        last = (div_ceil(last.0, 2), div_ceil(last.1, 2));

        down_sample_nodes(
            density_directory,
            first,
            last,
            lod,
            texture_size,
            0,
            ImageFormat::LUMA16,
        );
    }
}
