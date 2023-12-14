use bevy::{asset::load_internal_asset, prelude::*};

const PREPROCESSING_SHADER: Handle<Shader> = Handle::weak_from_u128(234753841217987793618);
pub(crate) const SPLIT_TILE_SHADER: Handle<Shader> = Handle::weak_from_u128(1284335798564325835856);
pub(crate) const STITCH_NODES_SHADER: Handle<Shader> =
    Handle::weak_from_u128(7314687437789154378358);
pub(crate) const DOWNSAMPLE_SHADER: Handle<Shader> =
    Handle::weak_from_u128(84231267615315384379763);

pub(crate) fn load_preprocess_shaders(app: &mut App) {
    load_internal_asset!(
        app,
        PREPROCESSING_SHADER,
        "preprocessing.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(app, SPLIT_TILE_SHADER, "split_tile.wgsl", Shader::from_wgsl);
    load_internal_asset!(
        app,
        STITCH_NODES_SHADER,
        "stitch_nodes.wgsl",
        Shader::from_wgsl
    );
    load_internal_asset!(app, DOWNSAMPLE_SHADER, "downsample.wgsl", Shader::from_wgsl);
}
