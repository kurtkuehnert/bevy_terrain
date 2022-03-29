use image::{ImageBuffer, Luma};
use std::fs;

fn parse_height(value: &str) -> u16 {
    let max_height = 1000.0;

    let height: f32 = value.parse().expect("Could not parse value of file.");
    (height / max_height * u16::MAX as f32) as u16
}

pub fn parse_file() {
    let dimension: u32 = 2000;

    let data = fs::read_to_string("assets/input/test3.xyz").expect("Unable to open file.");
    let data: Vec<u16> = data
        .split_whitespace()
        .skip(2)
        .step_by(3)
        .map(parse_height)
        .collect();

    assert_eq!(data.len() as u32, dimension * dimension);

    let section: ImageBuffer<Luma<u16>, Vec<u16>> =
        ImageBuffer::from_vec(dimension, dimension, data).unwrap();

    section.save("assets/heightmaps/map3.png").unwrap();
}
