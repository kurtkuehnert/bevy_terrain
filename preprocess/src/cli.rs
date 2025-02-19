use crate::{
    dataset::{PreprocessDataType, PreprocessNoData},
    gdal_extension::ProgressCallback,
};
use bevy_terrain::prelude::*;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

const BAR_SIZE: u64 = 10000;

#[derive(Parser, Debug)]
#[command(name = "btpp", author, version, about)]
pub struct Cli {
    #[arg(required = true)]
    pub src_path: Vec<PathBuf>,
    #[arg(required = true)]
    // cloud be optional and use current directory, but this would be risky in combination with overwrite
    pub terrain_path: PathBuf,
    #[arg(default_value = None)]
    pub temp_path: Option<PathBuf>,

    #[arg(short, long, default_value_t = false)]
    pub overwrite: bool,
    #[arg(default_value = "source")]
    pub no_data: PreprocessNoData,
    #[arg(default_value = "source")]
    pub data_type: PreprocessDataType,
    #[arg(default_value_t = 16.0)]
    pub fill_radius: f32,
    #[arg(default_value_t = false)]
    pub create_mask: bool,

    #[arg(default_value = None)]
    pub lod_count: Option<u32>,

    #[arg(default_value = "height")]
    pub attachment_label: AttachmentLabel,
    #[arg(short, long = "ts", default_value_t = 512)]
    pub texture_size: u32,
    #[arg(short, long = "bs", default_value_t = 1)]
    pub border_size: u32,
    #[arg(short, long = "m", default_value_t = 1)]
    pub mip_level_count: u32,
    #[arg(default_value = "ru16")]
    pub format: AttachmentFormat,
}

pub(crate) struct PreprocessBar<'a> {
    name: String,
    bar: ProgressBar,
    callback: Box<ProgressCallback<'a>>,
}

impl PreprocessBar<'_> {
    pub(crate) fn new(name: String) -> Self {
        let bar = ProgressBar::new(BAR_SIZE).with_style(
            ProgressStyle::with_template(
                &(name.clone() + " dataset: {wide_bar} {percent} % [{elapsed}/{duration}])"),
            )
            .unwrap(),
        );

        let callback = Box::new({
            let progress_bar = bar.clone();
            move |completion| {
                progress_bar.set_position((completion * BAR_SIZE as f64) as u64);
                true
            }
        });

        Self {
            name,
            bar,
            callback,
        }
    }

    pub(crate) fn callback(&self) -> &ProgressCallback {
        self.callback.as_ref()
    }

    pub(crate) fn finish(&self) {
        self.bar.finish_and_clear();
        println!("{} took: {:?}", self.name, self.bar.elapsed());
    }
}
