use bevy::{asset::load_internal_asset, prelude::*};
pub(crate) const SPLIT_TILE_SHADER: Handle<Shader> = Handle::weak_from_u128(1284335798564325835856);

pub(crate) fn load_preprocess_shaders(app: &mut App) {
    load_internal_asset!(app, SPLIT_TILE_SHADER, "split_tile.wgsl", Shader::from_wgsl);
}
