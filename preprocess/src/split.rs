use crate::{
    dataset::{create_tile_dataset, FaceInfo, PreprocessContext},
    gdal_extension::{CountingProgressCallback, ProgressCallback, SharedReadOnlyDataset},
    result::{PreprocessError, PreprocessResult},
    stitch::stitch,
};
use bevy_terrain::math::TileCoordinate;
use gdal::raster::{Buffer, GdalType};
use glam::IVec2;
use itertools::{iproduct, Itertools};
use num::NumCast;
use rayon::prelude::*;
use std::collections::HashMap;

pub fn split_and_stitch<T: Copy + GdalType + PartialEq + NumCast>(
    faces: HashMap<u32, FaceInfo>,
    context: &PreprocessContext,
    progress_callback: Option<&ProgressCallback>,
) -> PreprocessResult<Vec<TileCoordinate>> {
    let mut input_tiles = Vec::new();
    let mut datasets = HashMap::new();

    for (&face, info) in &faces {
        let src_dataset = SharedReadOnlyDataset::new(&info.path);
        datasets.insert(face, src_dataset);

        let xy_start = info.pixel_start / context.attachment.center_size() as i32; // round down
        let xy_end = (info.pixel_end - 1) / context.attachment.center_size() as i32 + 1; // round up

        input_tiles.extend(
            iproduct!(xy_start.x..xy_end.x, xy_start.y..xy_end.y)
                .map(|(x, y)| TileCoordinate::new(face, info.lod, IVec2::new(x, y))),
        );
    }

    let count = 2 * input_tiles.len() as u64;
    let progress_callback = CountingProgressCallback::new(count, progress_callback);

    let output_tiles = split::<T>(&input_tiles, faces, datasets, context, &progress_callback)?;
    stitch::<T>(&output_tiles, context, &progress_callback)?;

    Ok(output_tiles)
}

fn split<T: Copy + GdalType + PartialEq + NumCast>(
    input_tiles: &[TileCoordinate],
    faces: HashMap<u32, FaceInfo>,
    datasets: HashMap<u32, SharedReadOnlyDataset>,
    context: &PreprocessContext,
    progress_callback: &CountingProgressCallback,
) -> PreprocessResult<Vec<TileCoordinate>> {
    input_tiles
        .par_iter()
        .map(|&tile_coordinate| {
            let src_dataset = datasets.get(&tile_coordinate.face).unwrap().get();
            let face = faces.get(&tile_coordinate.face).unwrap();

            let tile_pixel_start = tile_coordinate.xy * context.attachment.center_size() as i32;
            let tile_pixel_end = (tile_coordinate.xy + 1) * context.attachment.center_size() as i32;

            let copy_size =
                tile_pixel_end.min(face.pixel_end) - tile_pixel_start.max(face.pixel_start);
            let src_offset = (tile_pixel_start - face.pixel_start).max(IVec2::ZERO);
            let tile_offset = (face.pixel_start - tile_pixel_start).max(IVec2::ZERO)
                + context.attachment.border_size as i32;

            // print!("tile: {tile_xy}, ");
            // print!("window size: {}, ", copy_size);
            // print!("src window position: {}, ", src_offset);
            // print!("tile window position: {}, ", tile_offset);
            // println!();

            let mut has_data = false;

            let copy_buffers: Vec<Buffer<T>> = src_dataset
                .rasterbands()
                .map(|src_raster| {
                    let src_raster = src_raster?;

                    let copy_buffer = src_raster.read_as::<T>(
                        (src_offset.x as isize, src_offset.y as isize),
                        (copy_size.x as usize, copy_size.y as usize),
                        (copy_size.x as usize, copy_size.y as usize),
                        None,
                    )?;

                    let no_data_value = src_raster
                        .no_data_value()
                        .map(|v| T::from(v).ok_or(PreprocessError::NoDataOutOfRange))
                        .transpose()?;

                    has_data |= no_data_value.is_none()
                        || copy_buffer
                            .data()
                            .iter()
                            .any(|&value| value != no_data_value.unwrap());

                    Ok::<Buffer<T>, PreprocessError>(copy_buffer)
                })
                .try_collect()?;

            // only create the tile if it actually contains data
            if has_data {
                let tile_dataset = create_tile_dataset::<T>(tile_coordinate, context).unwrap();

                for (band_index, mut copy_buffer) in copy_buffers.into_iter().enumerate() {
                    let mut tile_raster = tile_dataset.rasterband(band_index + 1)?;

                    tile_raster.write::<T>(
                        (tile_offset.x as isize, tile_offset.y as isize),
                        (copy_size.x as usize, copy_size.y as usize),
                        &mut copy_buffer,
                    )?;
                }
            }

            progress_callback.increment();

            Ok::<Option<TileCoordinate>, PreprocessError>(has_data.then(|| tile_coordinate))
        })
        .filter_map(Result::transpose)
        .collect::<PreprocessResult<Vec<TileCoordinate>>>()
}
