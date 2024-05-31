//! This crate provides the ability to render beautiful height-field terrains of any size.
//! This is achieved in extensible and modular manner, so that the terrain data
//! can be accessed from nearly anywhere (systems, shaders) [^note].
//!
//! # Background
//! There are three critical questions that each terrain renderer has to solve:
//!
//! ## How to store, manage and access the terrain data?
//! Each terrain has different types of textures associated with it.
//! For example a simple one might only need height and albedo information.
//! Because terrains can be quite large the space required for all of these so called
//! attachments, can/should not be stored in RAM and VRAM all at once.
//! Thus they have to be streamed in and out depending on the positions of the
//! viewers (cameras, lights, etc.).
//! Therefore the terrain is subdivided into a giant quadtree, whose nodes store their
//! section of these attachments.
//! This crate uses the chunked clipmap data structure, which consist of two pieces working together.
//! The wrapping [`Quadtree`](prelude::Quadtree) views together with
//! the [`NodeAtlas`](prelude::NodeAtlas) (the data structure
//! that stores all of the currently loaded data) can be used to efficiently retrieve
//! the best currently available data at any position for terrains of any size.
//! See the [`terrain_data`] module for more information.
//!
//! ## How to best approximate the terrain geometry?
//! Even a small terrain with a height map of 1000x1000 pixels would require 1 million vertices
//! to be rendered each frame per view, with an naive approach without an lod strategy.
//! To better distribute the vertices over the screen there exist many different algorithms.
//! This crate comes with its own default terrain geometry algorithm, called the
//! Uniform Distance-Dependent Level of Detail (UDLOD), which was developed with performance and
//! quality scalability in mind.
//! See the [`render`] module for more information.
//! You can also implement a different algorithm yourself and only use the terrain
//! data structures to solve the first question.
//!
//! ## How to shade the terrain?
//! The third and most important challenge of terrain rendering is the shading. This is a very
//! project specific problem and thus there does not exist a one-size-fits-all solution.
//! You can define your own terrain [Material](bevy::prelude::Material) and shader with all the
//! detail textures tailored to your application.
//! In the future this plugin will provide modular shader functions to make techniques like splat
//! mapping, triplane mapping, etc. easier.
//! Additionally a virtual texturing solution might be integrated to achieve better performance.
//!
//! [^note]: Some of these claims are not yet fully implemented.

pub mod big_space;
pub mod debug;
pub mod formats;
pub mod math;
pub mod plugin;
pub mod preprocess;
pub mod render;
pub mod terrain;
pub mod terrain_data;
pub mod terrain_view;
pub mod util;

pub mod prelude {
    //! `use bevy_terrain::prelude::*;` to import common components, bundles, and plugins.
    // #[doc(hidden)]
    pub use crate::{
        debug::{
            camera::{DebugCameraBundle, DebugCameraController},
            DebugTerrainMaterial, LoadingImages, TerrainDebugPlugin,
        },
        plugin::TerrainPlugin,
        preprocess::{
            preprocessor::Preprocessor,
            preprocessor::{PreprocessDataset, SphericalDataset},
            TerrainPreprocessPlugin,
        },
        render::render_pipeline::TerrainMaterialPlugin,
        terrain::{Terrain, TerrainBundle, TerrainConfig},
        terrain_data::{
            node_atlas::NodeAtlas, quadtree::Quadtree, sample_attachment, AttachmentConfig,
            AttachmentFormat,
        },
        terrain_view::{
            initialize_terrain_view, TerrainView, TerrainViewComponents, TerrainViewConfig,
        },
    };
}
