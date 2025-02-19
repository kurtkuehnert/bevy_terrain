use crate::cli::Cli;
use crate::result::{PreprocessError, PreprocessResult};
use bevy_terrain::math::TileCoordinate;
use bevy_terrain::terrain_data::{AttachmentConfig, AttachmentLabel};
use gdal::{
    programs::raster::build_vrt,
    raster::{ColorInterpretation, GdalDataType, GdalType, RasterCreationOptions},
    Dataset, DatasetOptions, DriverManager, GdalOpenFlags, GeoTransform,
};
use glam::{IVec2, U64Vec2};
use itertools::Itertools;
use std::{
    fs,
    ops::Not,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

#[derive(Debug, Clone, Copy)]
pub enum PreprocessNoData {
    Source,
    NoData(f64),
    Alpha, // Todo: implement this
}

impl FromStr for PreprocessNoData {
    type Err = PreprocessError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "source" => Ok(PreprocessNoData::Source),
            "alpha" => Ok(PreprocessNoData::Alpha),
            other => {
                let value = other.parse::<f64>()?;
                Ok(PreprocessNoData::NoData(value))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PreprocessDataType {
    Source,
    DataType(GdalDataType),
}

impl FromStr for PreprocessDataType {
    type Err = PreprocessError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "source" => Ok(PreprocessDataType::Source),
            other => Ok(PreprocessDataType::DataType(GdalDataType::from_name(
                other,
            )?)),
        }
    }
}

pub(crate) struct FaceInfo {
    pub(crate) lod: u32,
    pub(crate) pixel_start: IVec2,
    pub(crate) pixel_end: IVec2,
    pub(crate) path: PathBuf,
}

pub struct PreprocessContext {
    pub(crate) data_type: GdalDataType,
    pub(crate) no_data_value: Option<f64>,
    pub(crate) rasterbands: Vec<RasterbandConfig>,
    pub(crate) tile_dir: PathBuf,
    pub(crate) temp_dir: PathBuf,
    pub(crate) fill_radius: f32,
    pub(crate) create_mask: bool,
    pub(crate) overwrite: bool,

    pub(crate) min_height: f32,
    pub(crate) max_height: f32,

    pub(crate) terrain_path: PathBuf,
    pub(crate) lod_count: Option<u32>,
    pub(crate) attachment_label: AttachmentLabel,
    pub(crate) attachment: AttachmentConfig,
}

impl PreprocessContext {
    pub fn from_cli(args: Cli) -> PreprocessResult<(Dataset, Self)> {
        let Cli {
            src_path,
            terrain_path,
            temp_path,
            overwrite,
            no_data,
            data_type,
            fill_radius,
            create_mask,
            lod_count,
            attachment_label,
            texture_size,
            border_size,
            mip_level_count,
            format,
        } = args;

        PreprocessContext::initialize(
            terrain_path,
            lod_count,
            attachment_label,
            AttachmentConfig {
                texture_size,
                border_size,
                mip_level_count,
                format,
            },
            src_path,
            temp_path,
            no_data,
            data_type,
            fill_radius,
            create_mask,
            overwrite,
        )
    }
    pub(crate) fn initialize(
        terrain_path: PathBuf,
        lod_count: Option<u32>,

        attachment_label: AttachmentLabel,
        attachment: AttachmentConfig,

        src_path: Vec<PathBuf>,
        temp_dir: Option<PathBuf>,
        no_data: PreprocessNoData,
        data_type: PreprocessDataType,
        fill_radius: f32,
        overwrite: bool,
        create_mask: bool,
    ) -> PreprocessResult<(Dataset, Self)> {
        let mut src_datasets = src_path
            .iter()
            .map(|src_path| {
                if src_path.is_dir() {
                    iter_directory(&src_path).collect_vec()
                } else {
                    vec![src_path.clone()]
                }
            })
            .flatten()
            .map(|path| Dataset::open(path).unwrap())
            .collect_vec();

        let src_dataset = if src_datasets.len() == 1 {
            src_datasets.remove(0)
        } else {
            build_vrt(None, &src_datasets, None)?
        };

        let data_type = match data_type {
            PreprocessDataType::Source => src_dataset.rasterband(1)?.band_type(),
            PreprocessDataType::DataType(data_type) => data_type,
        };

        let mut rasterbands = src_dataset
            .rasterbands()
            .map(|rasterband| {
                let rasterband = rasterband.unwrap();
                RasterbandConfig {
                    color_interpretation: rasterband.color_interpretation(),
                }
            })
            .collect_vec();

        let no_data_value = match no_data {
            PreprocessNoData::Source => src_dataset.rasterband(1)?.no_data_value(),
            PreprocessNoData::NoData(value) => Some(value),
            PreprocessNoData::Alpha => {
                rasterbands.push(RasterbandConfig {
                    color_interpretation: ColorInterpretation::AlphaBand,
                });
                None
            }
        };

        let tile_dir = terrain_path.join(&String::from(&attachment_label));

        let temp_dir = match temp_dir {
            None => tile_dir.join("temp"),
            Some(path) => path,
        };

        Ok((
            src_dataset,
            Self {
                data_type,
                no_data_value,
                rasterbands,
                tile_dir,
                temp_dir,
                fill_radius,
                overwrite,
                min_height: f32::MAX,
                max_height: f32::MIN,
                create_mask,

                attachment_label,
                attachment,
                terrain_path,
                lod_count,
            },
        ))
    }
}

pub(crate) struct RasterbandConfig {
    color_interpretation: ColorInterpretation,
}

pub(crate) fn load_tile_dataset_if_exists(
    tile_coordinate: TileCoordinate,
    context: &PreprocessContext,
) -> PreprocessResult<Option<Dataset>> {
    let tile_path = tile_coordinate.path(&context.tile_dir);

    let dataset = if tile_path.is_file() {
        Some(Dataset::open(tile_path)?)
    } else {
        None
    };

    Ok(dataset)
}

pub(crate) fn update_tile_dataset(
    tile_coordinate: TileCoordinate,
    context: &PreprocessContext,
) -> PreprocessResult<Dataset> {
    let tile_path = tile_coordinate.path(&context.tile_dir);

    Ok(Dataset::open_ex(
        tile_path,
        DatasetOptions {
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE,
            ..Default::default()
        },
    )?)
}

pub(crate) fn create_tile_dataset<T: Copy + GdalType>(
    tile_coordinate: TileCoordinate,
    context: &PreprocessContext,
) -> PreprocessResult<Dataset> {
    let tile_path = tile_coordinate.path(&context.tile_dir);

    fs::create_dir_all(&tile_path.parent().unwrap()).unwrap(); // make sure the parent directories do exist

    create_empty_dataset::<T>(
        &tile_path,
        U64Vec2::splat(context.attachment.texture_size as u64),
        None,
        context,
    )
}

pub(crate) fn create_empty_dataset<T: Copy + GdalType>(
    dst_path: &Path,
    size: U64Vec2,
    geo_transform: Option<GeoTransform>,
    context: &PreprocessContext,
) -> PreprocessResult<Dataset> {
    let driver = DriverManager::get_driver_by_name("GTiff")?;

    // Todo: consider copying the photometric info

    let options = RasterCreationOptions::from_iter(
        [
            "TILED=YES",
            "BLOCKXSIZE=512",
            "BLOCKYSIZE=512",
            //  "SPARSE_OK=TRUE",
            "INTERLEAVE=PIXEL", // Todo: benchmark pixel vs band
        ]
        .into_iter(),
    );

    let mut dst = driver.create_with_band_type_with_options::<T, _>(
        dst_path,
        size.x as _,
        size.y as _,
        context.rasterbands.len(),
        &options,
    )?;

    if let Some(geo_transform) = geo_transform {
        dst.set_geo_transform(&geo_transform)?;
    }

    for (i, band) in context.rasterbands.iter().enumerate() {
        let mut dst_band = dst.rasterband(i + 1)?;
        dst_band.set_no_data_value(context.no_data_value)?;
        dst_band.set_color_interpretation(
            ColorInterpretation::from_c_int(band.color_interpretation.c_int()).unwrap(),
        )?;
    }

    Ok(dst)
}

pub fn delete_directory(directory: &Path) {
    // This method has issues with deleting hidden files on MacOS
    // let _ = fs::remove_dir_all(directory).unwrap();

    Command::new("rm")
        .arg("-rf")
        .arg(directory)
        .output()
        .unwrap();
}

pub fn clear_directory(directory: &Path) {
    delete_directory(directory);
    fs::create_dir_all(directory).unwrap();
}

pub fn iter_directory(directory: &Path) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(directory).unwrap().filter_map(|entry| {
        let path = entry.unwrap().path();

        path.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("._")
            .not()
            .then_some(path)
    })
}
