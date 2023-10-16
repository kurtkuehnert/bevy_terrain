use crate::{
    preprocess::file_io::{format_node_path, iterate_directory, load_image, save_image},
    terrain_data::{AttachmentConfig, NodeCoordinate, NodeId},
};
use image::{DynamicImage, ImageBuffer, LumaA};

pub(crate) fn height_to_minmax(
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
