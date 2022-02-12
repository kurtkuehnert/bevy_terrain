use crate::terrain::TerrainConfig;
use bevy::prelude::UVec2;
use image::{ImageBuffer, Luma};
use itertools::iproduct;
use std::path::Path;

pub fn generate_node_textures<P>(config: &TerrainConfig, source_path: P, output_path: &str)
where
    P: AsRef<Path>,
{
    let source = image::open(source_path).unwrap();
    let source = source.as_luma16().unwrap();

    let map_size: UVec2 = source.dimensions().into();

    assert_eq!(map_size, config.terrain_size);

    let section_size = (config.chunk_size + 1) as u32;

    for lod in 0..config.lod_count {
        let node_count = config.nodes_per_area(lod); // number of nodes per area
        let node_size = config.node_size(lod); // offset in the source image
        let mapping = 1 << lod; // pixel to pixel ratio

        // for every node of the current lod sample a new selection and save it
        for (y, x) in iproduct!(
            0..node_count * config.area_count.y,
            0..node_count * config.area_count.x
        ) {
            let node_id = TerrainConfig::node_id(lod, x, y);
            let section =
                sample_section(source, x * node_size, y * node_size, section_size, mapping);

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
    source_x: u32,
    source_y: u32,
    section_size: u32,
    mapping: u32,
) -> ImageBuffer<Luma<u16>, Vec<u16>> {
    let mut section = ImageBuffer::new(section_size, section_size);

    let (width, height) = source.dimensions();
    let sample_count = (mapping as f64).powf(2.0);

    for (section_x, section_y, pixel) in section.enumerate_pixels_mut() {
        let tx = source_x + section_x * mapping;
        let ty = source_y + section_y * mapping;

        *pixel = if tx == width || ty == height {
            Luma([0])
        } else {
            let value: f64 = iproduct!(0..mapping, 0..mapping)
                .map(|(offset_x, offset_y)| {
                    source.get_pixel(tx + offset_x, ty + offset_y).0[0] as f64
                })
                .sum();

            Luma([(value / sample_count) as u16])
        }
    }

    section
}
