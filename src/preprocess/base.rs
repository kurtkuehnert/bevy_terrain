use crate::{
    data_structures::{calc_node_id, AttachmentFormat},
    preprocess::{
        attachment::{down_sample_nodes, preprocess_attachment},
        format_path, load_node, AttachmentConfig, BaseConfig, TileConfig, UVec2Utils,
    },
    TerrainConfig, UVec2, Vec3,
};
use image::{DynamicImage, ImageBuffer, Luma};
use std::fs;

fn height_to_density(
    height_node: &DynamicImage,
    center_size: u32,
    border_size: u32,
    height: f32,
) -> DynamicImage {
    let height_node = height_node.as_luma16().unwrap();

    let density_node = ImageBuffer::from_fn(center_size, center_size, |x, y| {
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

fn density_chunks(
    height_directory: &str,
    density_directory: &str,
    first: UVec2,
    last: UVec2,
    center_size: u32,
    border_size: u32,
    height: f32,
) {
    for (x, y) in first.product(last) {
        let node_id = calc_node_id(0, x, y);
        let density_file_path = format!("{density_directory}/{node_id}.png");
        let height_file_path = format!("{height_directory}/{node_id}.png");

        let height_node = load_node(
            &height_file_path,
            center_size,
            border_size,
            AttachmentFormat::LUMA16,
        );

        let density_node = height_to_density(&height_node, center_size, border_size, height);

        density_node
            .save(density_file_path)
            .expect("Could not save file.");
    }
}

fn preprocess_density(
    height_directory: &str,
    density_directory: &str,
    first: UVec2,
    last: UVec2,
    height: f32,
    lod_count: u32,
    base_lod: u32,
    center_size: u32,
    border_size: u32,
) {
    let _ = fs::remove_dir_all(density_directory);
    fs::create_dir_all(density_directory).unwrap();

    density_chunks(
        height_directory,
        density_directory,
        first,
        last,
        center_size,
        border_size,
        height,
    );

    let mut first = first;
    let mut last = last;

    for lod in (base_lod + 1)..lod_count {
        first = first.div_floor(2);
        last = last.div_ceil(2);

        down_sample_nodes(
            density_directory,
            first,
            last,
            lod,
            center_size,
            0,
            AttachmentFormat::LUMA16,
        );
    }
}

// Todo: user should be able to set the resolution of the density attachment independently
pub(crate) fn preprocess_base(config: &TerrainConfig, tile: &TileConfig, base: &BaseConfig) {
    let height_directory = format_path(config.path, "height");
    let density_directory = format_path(config.path, "density");

    let height_attachment = AttachmentConfig {
        name: "height",
        center_size: base.center_size,
        border_size: 2,
        format: AttachmentFormat::LUMA16,
    };

    let (first, last) = preprocess_attachment(config, tile, &height_attachment);

    preprocess_density(
        &height_directory,
        &density_directory,
        first,
        last,
        config.height,
        config.lod_count,
        tile.lod,
        base.center_size,
        2,
    );
}
