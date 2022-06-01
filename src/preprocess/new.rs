use crate::quadtree::Node;
use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, Luma, RgbImage, RgbaImage};
use itertools::iproduct;

pub fn split_tile(
    input_file_path: &str,
    output_directory: &str,
    offset: (u32, u32),
    lod: u32,
    texture_size: u32,
    border_size: u32,
) {
    let tile = image::open(input_file_path).unwrap();

    let dimensions = tile.dimensions();

    // first and last chunk coordinate
    let first = (offset.0 / texture_size, offset.1 / texture_size);
    let last = (
        ((offset.0 + dimensions.0 + border_size) as f32 / texture_size as f32).ceil() as u32,
        ((offset.1 + dimensions.1 + border_size) as f32 / texture_size as f32).ceil() as u32,
    );

    for (x, y) in iproduct!(first.0..last.0, first.1..last.1) {
        let node_id = Node::id(lod, x, y);
        let file_path = format!("{output_directory}/{node_id}.png");

        let mut chunk = if let Ok(output) = image::open(&file_path) {
            output
        } else {
            let size = texture_size + 2 * border_size;
            match tile {
                DynamicImage::ImageRgb8(_) => DynamicImage::from(RgbImage::new(size, size)),
                DynamicImage::ImageRgba8(_) => DynamicImage::from(RgbaImage::new(size, size)),
                DynamicImage::ImageLuma16(_) => {
                    DynamicImage::from(<ImageBuffer<Luma<u16>, _>>::new(size, size))
                }
                _ => {
                    todo!("Add remaining formats.")
                }
            }
        };

        let dx = (offset.0 + border_size) as i64 - (x * texture_size) as i64;
        let dy = (offset.1 + border_size) as i64 - (y * texture_size) as i64;

        imageops::overlay(&mut chunk, &tile, dx, dy);
        chunk.save(&file_path).expect("Could not save file.");
    }
}
