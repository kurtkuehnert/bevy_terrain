use crate::config::TerrainConfig;

use crate::quadtree::Node;
use bevy::utils::HashMap;
use image::io::Reader;
use image::{DynamicImage, GenericImageView, ImageBuffer, Luma, Rgba, RgbaImage};
use itertools::iproduct;
use ron::to_string;
use std::{fs, path::Path};

struct NodeInfo {
    height_data: ImageBuffer<Luma<u16>, Vec<u16>>,
    min_height: u16,
    max_height: u16,
}

pub fn generate_node_textures<P>(config: &TerrainConfig, source_path: P, output_path: P)
where
    P: AsRef<Path>,
{
    let source = image::open(source_path).unwrap();
    let source = source.as_luma16().unwrap();

    let mut min_max_map = HashMap::<u32, (u16, u16)>::new();

    for lod in 0..config.lod_count {
        let node_count = config.nodes_per_area(lod); // number of nodes per area
        let node_size = config.node_size(lod); // offset in the source image
        let stride = 1 << lod; // pixel to pixel ratio

        // for every node of the current lod sample a new selection and save it
        for (y, x) in iproduct!(
            0..node_count * config.area_count.y,
            0..node_count * config.area_count.x
        ) {
            let node_id = Node::id(lod, x, y);
            let node = sample_node(
                source,
                x * node_size,
                y * node_size,
                config.chunk_size,
                stride,
            );

            min_max_map.insert(node_id, (node.min_height, node.max_height));

            let mut path = output_path.as_ref().join(&node_id.to_string());
            path.set_extension("png");
            node.height_data.save(path).unwrap();
        }
    }

    let data = to_string(&min_max_map).unwrap();
    fs::write(output_path.as_ref().join("min_max.ron"), data).expect("Unable to write file");
}

fn sample_node(
    source: &ImageBuffer<Luma<u16>, Vec<u16>>,
    origin_x: u32,
    origin_y: u32,
    texture_size: u32,
    stride: u32,
) -> NodeInfo {
    let mut node = NodeInfo {
        height_data: ImageBuffer::new(texture_size, texture_size),
        min_height: u16::MAX,
        max_height: 0,
    };

    let (width, height) = source.dimensions();
    let sample_count = (stride as f64).powf(2.0);

    for (node_x, node_y, pixel) in node.height_data.enumerate_pixels_mut() {
        let source_x = origin_x + node_x * stride;
        let source_y = origin_y + node_y * stride;

        if source_x < width && source_y < height {
            let value = (iproduct!(0..stride, 0..stride)
                .map(|(offset_x, offset_y)| {
                    source.get_pixel(source_x + offset_x, source_y + offset_y).0[0] as f64
                })
                .sum::<f64>()
                / sample_count) as u16;

            node.min_height = node.min_height.min(value);
            node.max_height = node.max_height.max(value);

            *pixel = Luma([value])
        }
    }

    node
}

pub fn generate_albedo_textures<P>(config: &TerrainConfig, source_path: P, output_path: P)
where
    P: AsRef<Path>,
{
    let mut reader = Reader::open(source_path).unwrap();
    reader.no_limits();

    let source = reader.decode().unwrap();
    // let source = source.as_rgba8().unwrap();

    for lod in 0..config.lod_count {
        let node_count = config.nodes_per_area(lod); // number of nodes per area
        let node_size = config.node_size(lod); // offset in the source image
        let stride = 1 << lod; // pixel to pixel ratio

        // for every node of the current lod sample a new selection and save it
        for (y, x) in iproduct!(
            0..node_count * config.area_count.y,
            0..node_count * config.area_count.x
        ) {
            let node_id = Node::id(lod, x, y);
            let albedo = sample_albedo(&source, x * node_size, y * node_size, 128, stride);

            let mut path = output_path.as_ref().join(&node_id.to_string());
            path.set_extension("png");
            albedo.save(path).unwrap();
        }
    }
}

fn sample_albedo(
    source: &DynamicImage,
    origin_x: u32,
    origin_y: u32,
    texture_size: u32,
    stride: u32,
) -> RgbaImage {
    let mut albedo = RgbaImage::new(texture_size, texture_size);

    let (width, height) = source.dimensions();
    let sample_count = (stride as f64).powf(2.0);

    for (node_x, node_y, pixel) in albedo.enumerate_pixels_mut() {
        let source_x = origin_x + node_x * stride;
        let source_y = origin_y + node_y * stride;

        if source_x < width && source_y < height {
            // let _value = (iproduct!(0..stride, 0..stride)
            //     .map(|(offset_x, offset_y)| {
            //         source.get_pixel(source_x + offset_x, source_y + offset_y).0[0] as f64
            //     })
            //     .sum::<f64>()
            //     / sample_count) as u16;

            let value = source.get_pixel(source_x, source_y).0;

            *pixel = Rgba(value)
        }
    }

    albedo
}
