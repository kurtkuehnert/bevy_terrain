use crate::preprocess::mip_maps::generate_mip_maps;
use crate::{
    preprocess::{
        down_sample::{down_sample_layer, linear, minmax},
        file_io::{
            format_directory, format_node_path, iterate_directory, load_image, reset_directory,
            save_image,
        },
        split::split_tiles,
        stitch::stitch_layer,
        BaseConfig, TileConfig, UVec2Utils,
    },
    terrain_data::{AttachmentConfig, NodeCoordinate, NodeId},
    TerrainConfig,
};
use image::{DynamicImage, ImageBuffer, LumaA};

fn height_to_minmax(
    height_directory: &str,
    minmax_directory: &str,
    height_attachment: &AttachmentConfig,
    minmax_attachment: &AttachmentConfig,
) {
    for (height_name, height_path) in iterate_directory(height_directory) {
        let coord = NodeCoordinate::from(height_name.parse::<NodeId>().unwrap());

        if coord.lod != 0 {
            continue;
        }

        let minmax_path = format_node_path(minmax_directory, coord.lod, coord.x, coord.y);

        let height_image = load_image(&height_path, height_attachment.file_format).unwrap();
        let height_image = height_image.as_luma16().unwrap();

        let minmax_image = DynamicImage::from(ImageBuffer::from_fn(
            height_image.width(),
            height_image.height(),
            |x, y| {
                let value = height_image.get_pixel(x, y).0[0];

                LumaA([value, value])
            },
        ));

        save_image(&minmax_path, &minmax_image, minmax_attachment);
    }
}

pub(crate) fn preprocess_base(config: &TerrainConfig, tile: &TileConfig, base: &BaseConfig) {
    let height_attachment = base.height_attachment();
    let minmax_attachment = base.minmax_attachment();

    let height_directory = format_directory(&config.path, "height");
    let minmax_directory = format_directory(&config.path, "minmax");

    reset_directory(&height_directory);
    reset_directory(&minmax_directory);

    let temp = split_tiles(&height_directory, tile, &height_attachment);

    let (mut first, mut last) = temp;

    for lod in 1..config.lod_count {
        first = first.div_floor(2);
        last = last.div_ceil(2);

        down_sample_layer(
            linear,
            &height_directory,
            &height_attachment,
            lod,
            first,
            last,
        );
        stitch_layer(&height_directory, &height_attachment, lod, first, last);
    }

    height_to_minmax(
        &height_directory,
        &minmax_directory,
        &height_attachment,
        &minmax_attachment,
    );

    generate_mip_maps(&height_directory, &height_attachment);

    let (mut first, mut last) = temp;

    for lod in 1..config.lod_count {
        first = first.div_floor(2);
        last = last.div_ceil(2);

        down_sample_layer(
            minmax,
            &minmax_directory,
            &minmax_attachment,
            lod,
            first,
            last,
        );
        stitch_layer(&minmax_directory, &minmax_attachment, lod, first, last);
    }

    generate_mip_maps(&minmax_directory, &minmax_attachment);
}

pub(crate) fn preprocess_attachment(
    config: &TerrainConfig,
    tile: &TileConfig,
    attachment: &AttachmentConfig,
) {
    let directory = format_directory(&config.path, &attachment.name);

    reset_directory(&directory);

    let (mut first, mut last) = split_tiles(&directory, tile, attachment);

    for lod in 1..config.lod_count {
        first = first.div_floor(2);
        last = last.div_ceil(2);

        down_sample_layer(linear, &directory, attachment, lod, first, last);
        stitch_layer(&directory, attachment, lod, first, last);
    }

    generate_mip_maps(&directory, attachment);
}
