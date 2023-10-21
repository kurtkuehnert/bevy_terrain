use crate::preprocess::{file_io::load_image, FileFormat, R16Image};
use image::{DynamicImage, GenericImage};

pub fn create_cube_map() {
    let size = 1000;
    let mut cube_map = DynamicImage::from(R16Image::new(size, 6 * size));

    for i in 0..6 {
        let path = format!("assets/textures/earth_1k/earth_{}.png", i);

        let image = load_image(&path, FileFormat::PNG).unwrap();

        cube_map.copy_from(&image, 0, i * size).unwrap();
    }

    cube_map.save("assets/textures/earth_cube.png").unwrap();
}
