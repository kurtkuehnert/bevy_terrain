use crate::{
    dataset::{create_tile_dataset, load_tile_dataset_if_exists, PreprocessContext},
    gdal_extension::{CountingProgressCallback, ProgressCallback},
    result::{PreprocessError, PreprocessResult},
    stitch::stitch,
};
use bevy_terrain::math::TileCoordinate;
use gdal::raster::{Buffer, GdalType, RasterBand, ResampleAlg};
use glam::IVec2;
use itertools::{izip, Itertools};
use num::NumCast;
use rayon::prelude::*;
use std::collections::HashSet;

pub fn downsample_and_stitch<T: Copy + GdalType + PartialEq + NumCast>(
    input_tiles: &[TileCoordinate],
    context: &PreprocessContext,
    progress_callback: Option<&ProgressCallback>,
) -> PreprocessResult<Vec<TileCoordinate>> {
    let tiles_to_downsample = compute_tiles_to_downsample(&input_tiles);

    let mut output_tiles = input_tiles.iter().copied().collect_vec();
    output_tiles.extend(tiles_to_downsample.iter().flatten());

    let count = 2 * tiles_to_downsample.iter().map(Vec::len).sum::<usize>() as u64;
    let progress_callback = CountingProgressCallback::new(count, progress_callback);

    for tiles in tiles_to_downsample {
        downsample::<T>(&tiles, context, &progress_callback)?;
        stitch::<T>(&tiles, context, &progress_callback)?;
    }

    Ok(output_tiles)
}

fn downsample<T: Copy + GdalType + PartialEq + NumCast>(
    input_tiles: &Vec<TileCoordinate>,
    context: &PreprocessContext,
    progress_callback: &CountingProgressCallback,
) -> PreprocessResult<()> {
    let child_center_size = context.attachment.center_size() / 2;
    let border_offset = (
        context.attachment.border_size as isize,
        context.attachment.border_size as isize,
    );
    let tile_size = (
        context.attachment.center_size() as usize,
        context.attachment.center_size() as usize,
    );
    let child_size = (child_center_size as usize, child_center_size as usize);

    input_tiles.par_iter().try_for_each(|&tile_coordinate| {
        let tile_dataset = create_tile_dataset::<T>(tile_coordinate, context)?;

        let mut tile_rasters: Vec<RasterBand> = tile_dataset.rasterbands().try_collect()?;

        let mut tile_buffers: Vec<Buffer<T>> = tile_rasters
            .iter()
            .map(|tile_raster| {
                let tile_buffer =
                    tile_raster.read_as::<T>(border_offset, tile_size, tile_size, None)?;
                Ok::<Buffer<T>, PreprocessError>(tile_buffer)
            })
            .try_collect()?;

        for child_coordinate in tile_coordinate.children() {
            if let Some(child_dataset) = load_tile_dataset_if_exists(child_coordinate, context)? {
                for (child_raster, tile_raster, tile_buffer) in izip!(
                    child_dataset.rasterbands(),
                    &mut tile_rasters,
                    &mut tile_buffers
                ) {
                    let child_raster = child_raster?;
                    let no_data_value = tile_raster.no_data_value().map(|v| T::from(v).unwrap());

                    let child_buffer = child_raster.read_as::<T>(
                        border_offset,
                        tile_size,
                        child_size,
                        Some(ResampleAlg::Bilinear),
                    )?;

                    for ((child_y, child_x), &child_value) in
                        child_buffer.to_array()?.indexed_iter()
                    {
                        let tile_xy = IVec2::new(child_x as i32, child_y as i32)
                            + child_center_size as i32 * (child_coordinate.xy % 2);

                        if no_data_value.is_none() || child_value != no_data_value.unwrap() {
                            tile_buffer[(tile_xy.y as usize, tile_xy.x as usize)] = child_value;
                        }
                    }
                }
            }
        }

        for (tile_raster, tile_buffer) in izip!(&mut tile_rasters, &mut tile_buffers) {
            tile_raster.write::<T>(border_offset, tile_size, tile_buffer)?;
        }

        progress_callback.increment();

        Ok::<(), PreprocessError>(())
    })
}

fn compute_tiles_to_downsample(input_tiles: &[TileCoordinate]) -> Vec<Vec<TileCoordinate>> {
    let mut tiles_to_downsample = input_tiles
        .iter()
        .filter_map(|tile| tile.parent())
        .collect::<HashSet<_>>();

    let mut new_tiles = tiles_to_downsample.clone();

    while !new_tiles.is_empty() {
        for tile in new_tiles.drain().collect_vec() {
            if let Some(parent) = tile.parent() {
                if tiles_to_downsample.insert(parent) {
                    new_tiles.insert(parent);
                }
            }
        }
    }

    let tiles = tiles_to_downsample.drain().collect_vec();
    let max_lod = tiles.iter().fold(0, |max_lod, tile_coordinate| {
        max_lod.max(tile_coordinate.lod)
    });

    (0..=max_lod)
        .rev()
        .map(|lod| {
            tiles
                .iter()
                .filter(|tile_coordinate| tile_coordinate.lod == lod)
                .copied()
                .collect_vec()
        })
        .collect_vec()
}
