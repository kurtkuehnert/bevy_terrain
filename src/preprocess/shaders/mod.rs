use crate::util::InternalShaders;
use bevy::{asset::embedded_asset, prelude::*};

pub(crate) const SPLIT_SHADER: &str = "embedded://bevy_terrain/preprocess/shaders/split.wgsl";
pub(crate) const STITCH_SHADER: &str = "embedded://bevy_terrain/preprocess/shaders/stitch.wgsl";
pub(crate) const DOWNSAMPLE_SHADER: &str =
    "embedded://bevy_terrain/preprocess/shaders/downsample.wgsl";

pub(crate) fn load_preprocess_shaders(app: &mut App) {
    embedded_asset!(app, "preprocessing.wgsl");
    embedded_asset!(app, "split.wgsl");
    embedded_asset!(app, "stitch.wgsl");
    embedded_asset!(app, "downsample.wgsl");

    InternalShaders::load(
        app,
        &["embedded://bevy_terrain/preprocess/shaders/preprocessing.wgsl"],
    );
}
