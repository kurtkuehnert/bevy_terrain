use crate::terrain::TerrainConfig;
use image::{ImageBuffer, Luma};
use itertools::iproduct;
use std::path::Path;

pub fn generate_node_textures<P>(config: &TerrainConfig, source_path: P, output_path: &str)
where
    P: AsRef<Path>,
{
    let source = image::open(source_path).unwrap();
    let source = source.as_luma16().unwrap();

    let (width, height) = source.dimensions();

    assert_eq!(width, config.width);
    assert_eq!(height, config.height);

    let node_size = (config.chunk_size + 1) as u32;

    for lod in 0..config.lod_count {
        let node_count = config.node_count(lod); // number of nodes per area
        let offset = config.area_size >> config.lod_count - lod - 1; // offset in the source image
        let mapping = 1 << lod; // pixel to pixel ratio

        // for every node of the current lod sample a new selection and save it
        for (y, x) in iproduct!(
            0..node_count * config.area_count_y,
            0..node_count * config.area_count_x
        ) {
            let node_id = config.calculate_node_id(lod, x, y);
            let section = sample_section(source, x * offset, y * offset, node_size, mapping);

            section
                .save(format!("{}{}.png", output_path, node_id))
                .unwrap();

            // section
            //     .save(format!("{}{}_{}_{}.png", output_path, x, y, lod))
            //     .unwrap();
        }
    }
}

fn sample_section(
    source: &ImageBuffer<Luma<u16>, Vec<u16>>,
    x: u32,
    y: u32,
    node_size: u32,
    mapping: u32,
) -> ImageBuffer<Luma<u16>, Vec<u16>> {
    let mut image_buffer = ImageBuffer::new(node_size, node_size);

    let (width, height) = source.dimensions();
    let sample_count = (mapping as f64).powf(2.0);

    for (px, py, pixel) in image_buffer.enumerate_pixels_mut() {
        let tx = x + px * mapping;
        let ty = y + py * mapping;

        *pixel = if tx == width || ty == height {
            Luma([0])
        } else {
            let value: f64 = iproduct!(0..mapping, 0..mapping)
                .map(|(ox, oy)| source.get_pixel(tx + ox, ty + oy).0[0] as f64)
                .sum();

            Luma([(value / sample_count) as u16])
        }
    }

    image_buffer
}
