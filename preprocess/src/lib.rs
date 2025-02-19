mod cli;
mod dataset;
mod downsample;
mod fill_no_data;
mod gdal_extension;
mod reproject;
mod result;
mod split;
mod stitch;
mod transformers;

use crate::{
    cli::PreprocessBar,
    dataset::{clear_directory, delete_directory, PreprocessContext},
    downsample::downsample_and_stitch,
    fill_no_data::create_mask_and_fill_no_data,
    reproject::reproject,
    split::split_and_stitch,
};
use bevy_terrain::prelude::*;
use gdal::{
    raster::{GdalDataType, GdalType},
    Dataset,
};
use num::NumCast;
use std::time::Instant;

pub mod prelude {
    pub use crate::{
        cli::Cli,
        dataset::{PreprocessContext, PreprocessDataType, PreprocessNoData},
        preprocess,
    };
}

fn preprocess_gen<T: Copy + GdalType + PartialEq + NumCast>(
    src_dataset: Dataset,
    context: &mut PreprocessContext,
) {
    if context.overwrite {
        clear_directory(&context.tile_dir);
    }

    clear_directory(&context.temp_dir);

    let start_preprocessing = Instant::now();

    let progress_bar = PreprocessBar::new("Reprojecting".to_string());
    let faces = reproject::<T>(src_dataset, context, Some(progress_bar.callback())).unwrap();
    progress_bar.finish();

    let progress_bar = PreprocessBar::new("Splitting".to_string());
    let tiles = split_and_stitch::<T>(faces, context, Some(progress_bar.callback())).unwrap();
    progress_bar.finish();

    let progress_bar = PreprocessBar::new("Downsampling".to_string());
    let tiles = downsample_and_stitch::<T>(&tiles, context, Some(progress_bar.callback())).unwrap();
    progress_bar.finish();

    let progress_bar = PreprocessBar::new("Filling".to_string());
    create_mask_and_fill_no_data(&tiles, context, Some(progress_bar.callback())).unwrap();
    progress_bar.finish();

    delete_directory(&context.temp_dir);

    save_terrain_config(tiles, &context);

    println!("Preprocessing took: {:?}", start_preprocessing.elapsed());
}

pub fn preprocess(src_dataset: Dataset, context: &mut PreprocessContext) {
    macro_rules! preprocess_gen {
        ($data_type:ty) => {
            preprocess_gen::<$data_type>(src_dataset, context)
        };
    }

    match context.data_type {
        GdalDataType::Unknown => panic!("Unknown data type!"),
        GdalDataType::UInt8 => preprocess_gen!(u8),
        GdalDataType::UInt16 => preprocess_gen!(u16),
        GdalDataType::UInt32 => preprocess_gen!(u32),
        GdalDataType::UInt64 => preprocess_gen!(u64),
        GdalDataType::Int8 => preprocess_gen!(i8),
        GdalDataType::Int16 => preprocess_gen!(i16),
        GdalDataType::Int32 => preprocess_gen!(i32),
        GdalDataType::Int64 => preprocess_gen!(i64),
        GdalDataType::Float32 => preprocess_gen!(f32),
        GdalDataType::Float64 => preprocess_gen!(f64),
    };
}

fn save_terrain_config(tiles: Vec<TileCoordinate>, context: &PreprocessContext) {
    let file_path = context.terrain_path.join("config.tc.ron");

    let mut config = TerrainConfig::load_file(&file_path).unwrap_or_default();

    config.shape = TerrainShape::WGS84;
    config.path = context.terrain_path.to_str().unwrap().to_string();
    config.add_attachment(context.attachment_label.clone(), context.attachment.clone());

    if context.attachment_label == AttachmentLabel::Height {
        config.min_height = context.min_height;
        config.max_height = context.max_height;
        config.tiles = tiles;
        config.lod_count = context.lod_count.unwrap();
    }

    config.save_file(&file_path).unwrap();
}
