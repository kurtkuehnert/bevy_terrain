use crate::{
    preprocess::{
        attachment::preprocess_attachment, format_node_path, format_path, iterate_images,
        load_image, reset_directory, AttachmentConfig, BaseConfig, TileConfig,
    },
    terrain_data::{AttachmentFormat, NodeCoordinate, NodeId},
    TerrainConfig, Vec3,
};
use image::{DynamicImage, ImageBuffer, Luma};

fn height_to_density(
    height_image: &DynamicImage,
    lod: u32,
    center_size: u32,
    border_size: u32,
    height: f32,
) -> DynamicImage {
    let height_image = height_image.as_luma16().unwrap();

    let density_image = ImageBuffer::from_fn(center_size, center_size, |x, y| {
        let left = height_image
            .get_pixel(x + border_size - 1, y + border_size)
            .0[0] as f32
            / u16::MAX as f32;
        let up = height_image
            .get_pixel(x + border_size, y + border_size - 1)
            .0[0] as f32
            / u16::MAX as f32;
        let right = height_image
            .get_pixel(x + border_size + 1, y + border_size)
            .0[0] as f32
            / u16::MAX as f32;
        let down = height_image
            .get_pixel(x + border_size, y + border_size + 1)
            .0[0] as f32
            / u16::MAX as f32;

        let normal = Vec3::new(right - left, (2 << lod) as f32 / height, down - up).normalize();
        let slope = ((1.0 - normal.y) * u16::MAX as f32) as u16;

        Luma([slope])
    });

    DynamicImage::from(density_image)
}

fn preprocess_density(
    height_directory: &str,
    density_directory: &str,
    center_size: u32,
    height: f32,
) {
    reset_directory(&density_directory);

    for (height_name, height_path) in iterate_images(height_directory) {
        let coord = NodeCoordinate::from(height_name.parse::<NodeId>().unwrap());
        let density_path = format_node_path(density_directory, coord.lod, coord.x, coord.y);

        let height_image = load_image(&height_path).unwrap();

        let density_image = height_to_density(&height_image, coord.lod, center_size, 2, height);

        density_image
            .save(&density_path)
            .expect("Could not save node.");
    }
}

// Todo: user should be able to set the resolution of the density attachment independently
pub(crate) fn preprocess_base(config: &TerrainConfig, tile: &TileConfig, base: &BaseConfig) {
    let height_directory = format_path(&config.path, "height");
    let density_directory = format_path(&config.path, "density");

    let height_attachment = AttachmentConfig {
        name: "height".to_string(),
        center_size: base.center_size,
        border_size: 2,
        format: AttachmentFormat::LUMA16,
    };

    preprocess_attachment(config, tile, &height_attachment);

    preprocess_density(
        &height_directory,
        &density_directory,
        base.center_size,
        config.height,
    );
}
