use crate::{
    preprocess::{
        attachment::preprocess_attachment, attachment::stitch_layer, format_node_path, format_path,
        iterate_images, load_image, load_or_create_node, reset_directory, AttachmentConfig,
        BaseConfig, Rg16Image, TileConfig, UVec2Utils,
    },
    skip_fail,
    terrain_data::{AttachmentFormat, NodeCoordinate, NodeId},
    TerrainConfig,
};
use bevy::prelude::UVec2;
use image::{DynamicImage, ImageBuffer, LumaA};
use itertools::iproduct;

fn height_to_minmax(height_directory: &str, minmax_directory: &str) {
    for (height_name, height_path) in iterate_images(height_directory) {
        let coord = NodeCoordinate::from(height_name.parse::<NodeId>().unwrap());

        if coord.lod != 0 {
            continue;
        }

        let minmax_path = format_node_path(minmax_directory, coord.lod, coord.x, coord.y);

        let height_image = load_image(&height_path).unwrap();
        let height_image = height_image.as_luma16().unwrap();

        let minmax_image = DynamicImage::from(ImageBuffer::from_fn(
            height_image.width(),
            height_image.height(),
            |x, y| {
                let value = height_image.get_pixel(x, y).0[0];

                LumaA([value, value])
            },
        ));

        minmax_image
            .save(&minmax_path)
            .expect("Could not save node.");
    }
}

fn down_sample_minmax(
    node_image: &mut Rg16Image,
    child_image: &Rg16Image,
    attachment: &AttachmentConfig,
    child_x: u32,
    child_y: u32,
) {
    let child_size = attachment.center_size >> 1;

    let node_x = child_x * child_size + attachment.border_size;
    let node_y = child_y * child_size + attachment.border_size;

    for (x, y) in iproduct!(0..child_size, 0..child_size) {
        let mut min = u16::MAX;
        let mut max = u16::MIN;

        for (cx, cy) in iproduct!(0..2, 0..2) {
            let value = child_image
                .get_pixel(
                    (x << 1) + cx + attachment.border_size,
                    (y << 1) + cy + attachment.border_size,
                )
                .0;
            min = min.min(value[0]);
            max = max.max(value[1]);
        }

        let value = LumaA([min, max]);
        node_image.put_pixel(node_x + x, node_y + y, value);
    }
}

fn down_sample_layer(
    directory: &str,
    attachment: &AttachmentConfig,
    lod: u32,
    first: UVec2,
    last: UVec2,
) {
    for (x, y) in first.product(last) {
        let node_path = format_node_path(directory, lod, x, y);
        let mut node_image = load_or_create_node(&node_path, attachment);
        let mut node_image = node_image.as_mut_luma_alpha16().unwrap();

        for (cx, cy) in iproduct!(0..2, 0..2) {
            let child_path = format_node_path(directory, lod - 1, (x << 1) + cx, (y << 1) + cy);
            let mut child_image = skip_fail!(load_image(&child_path));
            let child_image = child_image.as_mut_luma_alpha16().unwrap();
            // Todo: if a child node is not available, we should fill the gap in the parent one
            // maybe this should not even be possible

            down_sample_minmax(&mut node_image, &child_image, attachment, cx, cy);
        }

        node_image.save(node_path).expect("Could not save node.");
    }
}

pub(crate) fn preprocess_base(config: &TerrainConfig, tile: &TileConfig, base: &BaseConfig) {
    let height_directory = format_path(&config.path, "height");
    let minmax_directory = format_path(&config.path, "minmax");

    let height_attachment = AttachmentConfig {
        name: "height".to_string(),
        center_size: base.center_size,
        border_size: base.border_size,
        format: AttachmentFormat::R16,
    };

    let minmax_attachment = AttachmentConfig {
        name: "minmax".to_string(),
        center_size: base.center_size,
        border_size: base.border_size,
        format: AttachmentFormat::Rg16,
    };

    let (mut first, mut last) = preprocess_attachment(config, tile, &height_attachment);

    reset_directory(&minmax_directory);

    height_to_minmax(&height_directory, &minmax_directory);

    for lod in 1..config.lod_count {
        first = first.div_floor(2);
        last = last.div_ceil(2);

        down_sample_layer(&minmax_directory, &minmax_attachment, lod, first, last);
        stitch_layer(&minmax_directory, &minmax_attachment, lod, first, last);
    }
}
