use crate::dataset::update_tile_dataset;
use crate::{
    dataset::PreprocessContext,
    gdal_extension::{fill_no_data, CountingProgressCallback, ProgressCallback},
    result::{PreprocessError, PreprocessResult},
};
use bevy_terrain::math::TileCoordinate;
use gdal::raster::Buffer;
use gdal::raster::{GdalDataType, GdalType};
use itertools::{izip, Itertools};
use rayon::prelude::*;

trait BitMask {
    fn apply(&self, mask: u8) -> Self;
}

impl BitMask for f32 {
    fn apply(&self, mask: u8) -> Self {
        f32::from_bits((self.to_bits() & !1) | ((mask != 0) as u32))
    }
}

impl BitMask for u8 {
    fn apply(&self, mask: u8) -> Self {
        (self & !1) | ((mask != 0) as u8)
    }
}

impl BitMask for u16 {
    fn apply(&self, mask: u8) -> Self {
        (self & !1) | ((mask != 0) as u16)
    }
}

impl BitMask for i16 {
    fn apply(&self, mask: u8) -> Self {
        (self & !1) | ((mask != 0) as i16)
    }
}

fn create_mask_and_fill_no_data_gen<T: GdalType + BitMask>(
    tiles: &[TileCoordinate],
    context: &PreprocessContext,
    progress_callback: Option<&ProgressCallback>,
) -> PreprocessResult<()> {
    let progress_callback = CountingProgressCallback::new(tiles.len() as u64, progress_callback);

    tiles.par_iter().try_for_each(|&tile| {
        let src_dataset = update_tile_dataset(tile, context)?;

        let masks: Vec<Buffer<u8>> = src_dataset
            .rasterbands()
            .map(|src_raster| {
                let raster = src_raster?;
                let mask = raster.open_mask_band()?;
                let mask_data = mask.read_band_as()?;

                Ok::<Buffer<u8>, PreprocessError>(mask_data)
            })
            .try_collect()?;

        fill_no_data(&src_dataset, context.fill_radius as f64)?;

        for (mask, band) in izip!(masks, src_dataset.rasterbands()) {
            let mut band = band?;
            let mut band_data: Buffer<f32> = band.read_band_as()?;

            for (&mask, value) in mask.data().iter().zip(band_data.data_mut()) {
                *value = value.apply(mask); // all valid pixels have LSB == 1, all invalid pixels have LSB == 0
            }

            band.write(
                (0, 0),
                (
                    context.attachment.texture_size as usize,
                    context.attachment.texture_size as usize,
                ),
                &mut band_data,
            )?;
        }

        progress_callback.increment();

        Ok::<(), PreprocessError>(())
    })
}

fn only_fill_no_data_gen<T: GdalType>(
    tiles: &[TileCoordinate],
    context: &PreprocessContext,
    progress_callback: Option<&ProgressCallback>,
) -> PreprocessResult<()> {
    let progress_callback = CountingProgressCallback::new(tiles.len() as u64, progress_callback);

    tiles.par_iter().try_for_each(|&tile| {
        let src_dataset = update_tile_dataset(tile, context)?;

        fill_no_data(&src_dataset, context.fill_radius as f64)?;

        progress_callback.increment();

        Ok::<(), PreprocessError>(())
    })
}

pub fn create_mask_and_fill_no_data(
    tiles: &[TileCoordinate],
    context: &PreprocessContext,
    progress_callback: Option<&ProgressCallback>,
) -> PreprocessResult<()> {
    macro_rules! create_mask_and_fill_no_data_gen {
        ($data_type:ty) => {
            create_mask_and_fill_no_data_gen::<$data_type>(tiles, context, progress_callback)
        };
    }

    macro_rules! only_fill_no_data_gen {
        ($data_type:ty) => {
            only_fill_no_data_gen::<$data_type>(tiles, context, progress_callback)
        };
    }

    if context.create_mask {
        match context.data_type {
            GdalDataType::Unknown => Err(PreprocessError::UnknownRasterbandDataType),
            GdalDataType::UInt8 => create_mask_and_fill_no_data_gen!(u8),
            GdalDataType::UInt16 => create_mask_and_fill_no_data_gen!(u16),
            GdalDataType::UInt32 => panic!("This is not supported."),
            GdalDataType::UInt64 => panic!("This is not supported."),
            GdalDataType::Int8 => panic!("This is not supported."),
            GdalDataType::Int16 => create_mask_and_fill_no_data_gen!(i16),
            GdalDataType::Int32 => panic!("This is not supported."),
            GdalDataType::Int64 => panic!("This is not supported."),
            GdalDataType::Float32 => create_mask_and_fill_no_data_gen!(f32),
            GdalDataType::Float64 => panic!("This is not supported."),
        }
    } else {
        match context.data_type {
            GdalDataType::Unknown => Err(PreprocessError::UnknownRasterbandDataType),
            GdalDataType::UInt8 => only_fill_no_data_gen!(u8),
            GdalDataType::UInt16 => only_fill_no_data_gen!(u16),
            GdalDataType::UInt32 => only_fill_no_data_gen!(u32),
            GdalDataType::UInt64 => only_fill_no_data_gen!(u64),
            GdalDataType::Int8 => only_fill_no_data_gen!(i8),
            GdalDataType::Int16 => only_fill_no_data_gen!(i16),
            GdalDataType::Int32 => only_fill_no_data_gen!(i32),
            GdalDataType::Int64 => only_fill_no_data_gen!(i64),
            GdalDataType::Float32 => only_fill_no_data_gen!(f32),
            GdalDataType::Float64 => only_fill_no_data_gen!(f64),
        }
    }
}
