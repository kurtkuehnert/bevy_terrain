use crate::terrain_data::AttachmentLabel;
use bevy::{asset::embedded_asset, prelude::*};
use itertools::Itertools;
pub const DEFAULT_VERTEX_SHADER: &str = "embedded://bevy_terrain/shaders/render/vertex.wgsl";
pub const DEFAULT_FRAGMENT_SHADER: &str = "embedded://bevy_terrain/shaders/render/fragment.wgsl";
pub const PREPARE_PREPASS_SHADER: &str =
    "embedded://bevy_terrain/shaders/tiling_prepass/prepare_prepass.wgsl";
pub const REFINE_TILES_SHADER: &str =
    "embedded://bevy_terrain/shaders/tiling_prepass/refine_tiles.wgsl";
// pub(crate) const SPLIT_SHADER: &str = "embedded://bevy_terrain/shaders/preprocess/split.wgsl";
// pub(crate) const STITCH_SHADER: &str = "embedded://bevy_terrain/shaders/preprocess/stitch.wgsl";
// pub(crate) const DOWNSAMPLE_SHADER: &str =
//     "embedded://bevy_terrain/shaders/preprocess/downsample.wgsl";
pub(crate) const PICKING_SHADER: &str = "embedded://bevy_terrain/shaders/picking.wgsl";
pub(crate) const DEPTH_COPY_SHADER: &str = "embedded://bevy_terrain/shaders/depth_copy.wgsl";

#[derive(Default, Resource)]
pub(crate) struct InternalShaders(Vec<Handle<Shader>>);

impl InternalShaders {
    pub(crate) fn load(app: &mut App, shaders: &[&'static str]) {
        let mut shaders = shaders
            .iter()
            .map(|&shader| app.world_mut().resource_mut::<AssetServer>().load(shader))
            .collect_vec();

        let mut internal_shaders = app.world_mut().resource_mut::<InternalShaders>();
        internal_shaders.0.append(&mut shaders);
    }
}

fn load_bindings_shader(app: &mut App, attachments: &[AttachmentLabel]) {
    let source = include_str!("bindings.wgsl");

    let source = (0..8).fold(source.to_string(), |src, i| {
        src.replacen(
            &format!("{{{i}}}"),
            &String::from(
                &attachments
                    .get(i)
                    .cloned()
                    .unwrap_or(AttachmentLabel::Empty(i)),
            ),
            2,
        )
    });

    let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
    let shader = shaders.add(Shader::from_wgsl(source, "bindings.wgsl"));

    let mut internal_shaders = app.world_mut().resource_mut::<InternalShaders>();
    internal_shaders.0.push(shader);
}

pub(crate) fn load_terrain_shaders(app: &mut App, attachments: &[AttachmentLabel]) {
    embedded_asset!(app, "types.wgsl");
    embedded_asset!(app, "attachments.wgsl");
    embedded_asset!(app, "functions.wgsl");
    embedded_asset!(app, "debug.wgsl");
    embedded_asset!(app, "render/vertex.wgsl");
    embedded_asset!(app, "render/fragment.wgsl");
    embedded_asset!(app, "tiling_prepass/prepare_prepass.wgsl");
    embedded_asset!(app, "tiling_prepass/refine_tiles.wgsl");
    embedded_asset!(app, "picking.wgsl");
    embedded_asset!(app, "depth_copy.wgsl");

    load_bindings_shader(app, attachments);

    InternalShaders::load(
        app,
        &[
            "embedded://bevy_terrain/shaders/types.wgsl",
            "embedded://bevy_terrain/shaders/attachments.wgsl",
            "embedded://bevy_terrain/shaders/functions.wgsl",
            "embedded://bevy_terrain/shaders/debug.wgsl",
            "embedded://bevy_terrain/shaders/render/vertex.wgsl",
            "embedded://bevy_terrain/shaders/render/fragment.wgsl",
        ],
    );
}

// pub(crate) fn load_preprocess_shaders(app: &mut App) {
//     embedded_asset!(app, "preprocess/preprocessing.wgsl");
//     embedded_asset!(app, "preprocess/split.wgsl");
//     embedded_asset!(app, "preprocess/stitch.wgsl");
//     embedded_asset!(app, "preprocess/downsample.wgsl");
//
//     InternalShaders::load(
//         app,
//         &["embedded://bevy_terrain/shaders/preprocess/preprocessing.wgsl"],
//     );
// }
