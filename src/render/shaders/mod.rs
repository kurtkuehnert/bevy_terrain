use crate::util::InternalShaders;
use bevy::{asset::embedded_asset, prelude::*};

pub const DEFAULT_SHADER: &str = "embedded://bevy_terrain/render/shaders/render/default.wgsl";
pub const PREPARE_INDIRECT_SHADER: &str =
    "embedded://bevy_terrain/render/shaders/compute/prepare_indirect.wgsl";
pub const REFINE_TILES_SHADER: &str =
    "embedded://bevy_terrain/render/shaders/compute/refine_tiles.wgsl";

pub(crate) fn load_terrain_shaders(app: &mut App) {
    embedded_asset!(app, "types.wgsl");
    embedded_asset!(app, "attachments.wgsl");
    embedded_asset!(app, "bindings.wgsl");
    embedded_asset!(app, "functions.wgsl");
    embedded_asset!(app, "debug.wgsl");
    embedded_asset!(app, "render/vertex.wgsl");
    embedded_asset!(app, "render/fragment.wgsl");
    embedded_asset!(app, "render/default.wgsl");
    embedded_asset!(app, "compute/prepare_indirect.wgsl");
    embedded_asset!(app, "compute/refine_tiles.wgsl");

    InternalShaders::load(
        app,
        &[
            "embedded://bevy_terrain/render/shaders/types.wgsl",
            "embedded://bevy_terrain/render/shaders/attachments.wgsl",
            "embedded://bevy_terrain/render/shaders/bindings.wgsl",
            "embedded://bevy_terrain/render/shaders/functions.wgsl",
            "embedded://bevy_terrain/render/shaders/debug.wgsl",
            "embedded://bevy_terrain/render/shaders/render/vertex.wgsl",
            "embedded://bevy_terrain/render/shaders/render/fragment.wgsl",
        ],
    );
}
