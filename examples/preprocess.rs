use bevy_terrain::preprocess::prelude::*;

// keep in sync with examples
const TERRAIN_SIZE: u32 = 1024;
const LOD_COUNT: u32 = 5;
const CHUNK_SIZE: u32 = 128;
const HEIGHT: f32 = 200.0;

fn main() {
    preprocess_tiles(
        "assets/terrain/source/height",
        "assets/terrain/data/height",
        0,
        LOD_COUNT,
        (0, 0),
        TERRAIN_SIZE,
        CHUNK_SIZE,
        2,
        ImageFormat::LUMA16,
    );
    preprocess_density(
        "assets/terrain/data/height",
        "assets/terrain/data/density",
        LOD_COUNT,
        (0, 0),
        (9, 9),
        CHUNK_SIZE,
        2,
        HEIGHT,
    );
    preprocess_tiles(
        "assets/terrain/source/albedo.png",
        "assets/terrain/data/albedo",
        0,
        LOD_COUNT,
        (0, 0),
        2 * TERRAIN_SIZE,
        2 * CHUNK_SIZE,
        1,
        ImageFormat::RGB,
    );
}
