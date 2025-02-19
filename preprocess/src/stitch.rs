use crate::{
    dataset::{load_tile_dataset_if_exists, update_tile_dataset, PreprocessContext},
    gdal_extension::CountingProgressCallback,
    result::{PreprocessError, PreprocessResult},
};
use bevy_terrain::math::{FaceRotation, TileCoordinate};
use gdal::raster::{Buffer, GdalType};
use gdal::Dataset;
use itertools::izip;
use ndarray::Axis;
use num::NumCast;
use rayon::prelude::*;

fn stitch_corners<T: Copy + GdalType + NumCast>(
    tile_dataset: &Dataset,
    dst_offsets: &[(isize, isize)],
    i: usize,
    context: &PreprocessContext,
) -> PreprocessResult<()> {
    // Cube corners should be filled with the average of the three adjacent pixels.
    // This assumes, that the side stitching has completed already.

    let dst_offset = dst_offsets[i];
    let border_size = context.attachment.border_size as usize;
    let offset_size = context.attachment.offset_size() as isize;

    let src_offset = (
        match dst_offset.0 {
            0 => border_size as isize,
            s if s == offset_size => offset_size - 1,
            s => s,
        },
        match dst_offset.1 {
            0 => border_size as isize,
            s if s == offset_size => offset_size - 1,
            s => s,
        },
    );

    let corner = i - 4;
    let corner_offsets: [[(isize, isize); 3]; 4] = [
        [(0, 0), (-1, 0), (0, -1)],
        [(0, 0), (1, 0), (0, -1)],
        [(0, 0), (1, 0), (0, 1)],
        [(0, 0), (-1, 0), (0, 1)],
    ];

    for raster in tile_dataset.rasterbands() {
        let mut raster = raster?;

        let corner_values = corner_offsets[corner].map(|offset| {
            Ok::<T, PreprocessError>(
                raster
                    .read_as::<T>(
                        (src_offset.0 + offset.0, src_offset.1 + offset.1),
                        (1, 1),
                        (1, 1),
                        None,
                    )
                    .unwrap()
                    .data()[0],
            )
        });

        // Todo: check for nodata

        let avg = corner_values
            .into_iter()
            .map(|v| v?.to_f64().ok_or(PreprocessError::TransformOperationFailed))
            .collect::<Result<Vec<f64>, _>>()?
            .into_iter()
            .sum::<f64>()
            / 3.0;

        let mut buffer = Buffer::new((1, 1), vec![T::from(avg).unwrap()]);

        raster.write::<T>(dst_offset, (border_size, border_size), &mut buffer)?;
    }

    Ok(())
}

fn neighbour_data<T: Copy + GdalType>(
    tile_dataset: &Dataset,
    neighbour_dataset: &Dataset,
    rotation: FaceRotation,
    i: usize,
    src_offsets: &[(isize, isize)],
    dst_offsets: &[(isize, isize)],
    sizes: &[(usize, usize)],
) -> PreprocessResult<()> {
    let size = sizes[i];
    let dst_offset = dst_offsets[i];
    let dst_size = size;

    let (src_offset, src_size) = match rotation {
        FaceRotation::Identical | FaceRotation::ShiftU | FaceRotation::ShiftV => {
            (src_offsets[i], size)
        }
        FaceRotation::RotateCW => {
            if i < 4 {
                (src_offsets[(i + 3) % 4], (size.1, size.0))
            } else {
                (src_offsets[4 + ((i + 3) % 4)], (size.1, size.0))
            }
        }
        FaceRotation::RotateCCW => {
            if i < 4 {
                (src_offsets[(i + 1) % 4], (size.1, size.0))
            } else {
                (src_offsets[4 + ((i + 1) % 4)], (size.1, size.0))
            }
        }
        FaceRotation::Backside => unreachable!(),
    };

    for (tile_raster, neighbour_raster) in
        izip!(tile_dataset.rasterbands(), neighbour_dataset.rasterbands())
    {
        let mut tile_raster = tile_raster?;
        let neighbour_raster = neighbour_raster?;

        let buffer = neighbour_raster.read_as::<T>(src_offset, src_size, src_size, None)?;

        let mut buffer = match rotation {
            FaceRotation::Identical | FaceRotation::ShiftU | FaceRotation::ShiftV => buffer,
            FaceRotation::RotateCW => {
                let mut array = buffer.to_array()?;
                array.swap_axes(0, 1);
                array.invert_axis(Axis(1));
                Buffer::from(array)
            }
            FaceRotation::RotateCCW => {
                let mut array = buffer.to_array()?;
                array.swap_axes(0, 1);
                array.invert_axis(Axis(0));
                Buffer::from(array)
            }
            FaceRotation::Backside => unreachable!(),
        };

        tile_raster.write::<T>(dst_offset, dst_size, &mut buffer)?;
    }

    Ok(())
}

pub(crate) fn stitch<T: Copy + GdalType + NumCast>(
    tiles: &[TileCoordinate],
    context: &PreprocessContext,
    progress_callback: &CountingProgressCallback,
) -> PreprocessResult<()> {
    let center_size = context.attachment.center_size() as usize;
    let border_size = context.attachment.border_size as usize;
    let offset_size = context.attachment.offset_size() as isize;

    let src_offsets: [(isize, isize); 8] = [
        (border_size as isize, center_size as isize),
        (border_size as isize, border_size as isize),
        (border_size as isize, border_size as isize),
        (center_size as isize, border_size as isize),
        (center_size as isize, center_size as isize),
        (border_size as isize, center_size as isize),
        (border_size as isize, border_size as isize),
        (center_size as isize, border_size as isize),
    ];

    let dst_offsets: [(isize, isize); 8] = [
        (border_size as isize, 0),
        (offset_size, border_size as isize),
        (border_size as isize, offset_size),
        (0, border_size as isize),
        (0, 0),
        (offset_size, 0),
        (offset_size, offset_size),
        (0, offset_size),
    ];

    let sizes: [(usize, usize); 8] = [
        (center_size, border_size),
        (border_size, center_size),
        (center_size, border_size),
        (border_size, center_size),
        (border_size, border_size),
        (border_size, border_size),
        (border_size, border_size),
        (border_size, border_size),
    ];

    tiles.par_iter().try_for_each(|&tile_coordinate| {
        let tile_dataset = update_tile_dataset(tile_coordinate, context)?;

        for (i, (neighbour_coordinate, rotation)) in tile_coordinate.neighbours(true).enumerate() {
            if let Some(neighbour_dataset) =
                load_tile_dataset_if_exists(neighbour_coordinate, context)?
            {
                neighbour_data::<T>(
                    &tile_dataset,
                    &neighbour_dataset,
                    rotation,
                    i,
                    &src_offsets,
                    &dst_offsets,
                    &sizes,
                )?;
            } else if i >= 4 {
                stitch_corners::<T>(&tile_dataset, &dst_offsets, i, context)?;
            };
        }

        progress_callback.increment();

        Ok::<(), PreprocessError>(())
    })
}
