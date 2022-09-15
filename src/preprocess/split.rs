use crate::{
    preprocess::{
        format_node_path, iterate_directory, load_image, load_or_create_node, save_node,
        TileConfig, UVec2Utils,
    },
    terrain_data::{AttachmentConfig, AttachmentFormat},
};
use bevy::prelude::*;
use image::{
    imageops::{self},
    DynamicImage,
};
use std::fs;

fn tile_to_node(
    node_image: &mut DynamicImage,
    tile_image: &DynamicImage,
    tile: &TileConfig,
    attachment: &AttachmentConfig,
    coord: UVec2,
) {
    let x =
        (tile.offset.x + attachment.border_size) as i64 - (coord.x * attachment.center_size) as i64;
    let y =
        (tile.offset.y + attachment.border_size) as i64 - (coord.y * attachment.center_size) as i64;

    match attachment.format {
        AttachmentFormat::Rgb8 => imageops::replace(
            node_image.as_mut_rgb8().unwrap(),
            tile_image.as_rgb8().unwrap(),
            x,
            y,
        ),
        AttachmentFormat::Rgba8 => imageops::replace(
            node_image.as_mut_rgba8().unwrap(),
            tile_image.as_rgba8().unwrap(),
            x,
            y,
        ),
        AttachmentFormat::R16 => imageops::replace(
            node_image.as_mut_luma16().unwrap(),
            tile_image.as_luma16().unwrap(),
            x,
            y,
        ),
        AttachmentFormat::Rg16 => imageops::replace(
            node_image.as_mut_luma_alpha16().unwrap(),
            tile_image.as_luma_alpha16().unwrap(),
            x,
            y,
        ),
    };
}

fn split_tile(directory: &str, tile: &TileConfig, attachment: &AttachmentConfig) {
    let tile_image = load_image(&tile.path).expect("Could not load tile.");

    // first and last node coordinate
    let first = tile.offset.div_floor(attachment.center_size);
    let last = (tile.offset + tile.size + attachment.border_size).div_ceil(attachment.center_size);

    for (x, y) in first.product(last) {
        let node_path = format_node_path(directory, tile.lod, x, y);

        let mut node_image = load_or_create_node(&node_path, attachment);

        tile_to_node(
            &mut node_image,
            &tile_image,
            tile,
            attachment,
            UVec2::new(x, y),
        );

        save_node(&node_path, &node_image, attachment.format);
    }
}

pub(crate) fn split_tiles(
    directory: &str,
    tile: &TileConfig,
    attachment: &AttachmentConfig,
) -> (UVec2, UVec2) {
    let (offset, size) = if fs::metadata(&tile.path).unwrap().is_dir() {
        let mut min_pos = UVec2::splat(u32::MAX);
        let mut max_pos = UVec2::splat(u32::MIN);

        for (tile_name, tile_path) in iterate_directory(&tile.path) {
            let mut parts = tile_name.split('_');
            parts.next();

            let coord = UVec2::new(
                parts.next().unwrap().parse::<u32>().unwrap(),
                parts.next().unwrap().parse::<u32>().unwrap(),
            );

            let tile = TileConfig {
                path: tile_path,
                offset: tile.offset + coord * tile.size,
                ..*tile
            };

            split_tile(directory, &tile, attachment);

            min_pos = min_pos.min(coord);
            max_pos = max_pos.max(coord);
        }

        let offset = tile.offset + min_pos * tile.size;
        let size = (1 + max_pos - min_pos) * tile.size;

        (offset, size)
    } else {
        split_tile(directory, tile, attachment);

        (tile.offset, UVec2::splat(tile.size))
    };

    let first = offset.div_floor(attachment.center_size);
    let last = (offset + size).div_ceil(attachment.center_size);

    (first, last)
}
