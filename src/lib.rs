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
//! The wrapping [`Quadtree`] views together with
//! the [`NodeAtlas`] (the data structure
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
//! You can define your own terrain [Material] and shader with all the
//! detail textures tailored to your application.
//! In the future this plugin will provide modular shader functions to make techniques like splat
//! mapping, triplane mapping, etc. easier.
//! Additionally a virtual texturing solution might be integrated to achieve better performance.
//!
//! [^note]: Some of these claims are not yet fully implemented.

extern crate core;

use crate::{
    attachment_loader::{finish_loading_attachment_from_disk, start_loading_attachment_from_disk},
    debug::DebugTerrain,
    formats::TDFPlugin,
    render::{
        compute_pipelines::{
            queue_terrain_compute_pipelines, TerrainComputeNode, TerrainComputePipelines,
        },
        culling::{prepare_and_queue_terrain_culling_bind_group, CullingBindGroup},
        render_pipeline::TerrainPipelineConfig,
        shaders::add_shader,
        terrain_data::{initialize_terrain_data, TerrainData},
        terrain_view_data::{
            extract_terrain_view_config, initialize_terrain_view_data, prepare_terrain_view_config,
            TerrainViewConfigUniform, TerrainViewData,
        },
    },
    terrain::{Terrain, TerrainComponents, TerrainConfig},
    terrain_data::{
        gpu_node_atlas::{
            extract_node_atlas, initialize_gpu_node_atlas, prepare_node_atlas, GpuNodeAtlas,
        },
        gpu_quadtree::{extract_quadtree, initialize_gpu_quadtree, prepare_quadtree, GpuQuadtree},
        node_atlas::{update_node_atlas, NodeAtlas},
        quadtree::{
            adjust_quadtree, compute_quadtree_request, update_height_under_viewer, Quadtree,
        },
    },
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
};
use bevy::{
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin, main_graph::node::CAMERA_DRIVER,
        render_graph::RenderGraph, render_resource::*, view::NoFrustumCulling, RenderApp,
        RenderSet,
    },
};

pub mod attachment_loader;
pub mod debug;
pub mod formats;
pub mod preprocess;
pub mod render;
pub mod terrain;
pub mod terrain_data;
pub mod terrain_view;

pub mod prelude {
    //! `use bevy_terrain::prelude::*;` to import common components, bundles, and plugins.
    // #[doc(hidden)]
    pub use crate::{
        attachment_loader::AttachmentFromDiskLoader,
        debug::{camera::DebugCamera, TerrainDebugPlugin},
        preprocess::{config::load_node_config, BaseConfig, Preprocessor, TileConfig},
        render::render_pipeline::TerrainMaterialPlugin,
        terrain::{Terrain, TerrainConfig},
        terrain_data::{
            node_atlas::NodeAtlas, quadtree::Quadtree, AttachmentConfig, AttachmentFormat,
            FileFormat,
        },
        terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
        TerrainBundle, TerrainPlugin,
    };
}

/// The components of a terrain.
///
/// Does not include loader(s) and a material.
#[derive(Bundle)]
pub struct TerrainBundle {
    terrain: Terrain,
    node_atlas: NodeAtlas,
    config: TerrainConfig,
    transform: Transform,
    global_transform: GlobalTransform,
    
    visibility_bundle: VisibilityBundle,
    no_frustum_culling: NoFrustumCulling,
}

impl TerrainBundle {
    /// Creates a new terrain bundle from the config.
    pub fn new(config: TerrainConfig) -> Self {
        Self {
            terrain: Terrain,
            node_atlas: NodeAtlas::from_config(&config),
            config,
            transform: default(),
            global_transform: default(),
            visibility_bundle: default(),
            no_frustum_culling: NoFrustumCulling,
        }
    }
}

/// The plugin for the terrain renderer.
pub struct TerrainPlugin {
    /// The number of terrain attachments.
    pub attachment_count: usize,
}

impl Default for TerrainPlugin {
    fn default() -> Self {
        Self {
            attachment_count: 2,
        }
    }
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
       
       

        app.add_plugins(TDFPlugin)
            .add_plugins(ExtractComponentPlugin::<Terrain>::default())
            .add_plugins(ExtractComponentPlugin::<TerrainView>::default())
            .init_resource::<TerrainViewComponents<Quadtree>>()
            .init_resource::<TerrainViewComponents<TerrainViewConfig>>()
            .add_systems(
                 Last,//was CoreSet::Last
                (
                    finish_loading_attachment_from_disk.before(update_node_atlas),
                    compute_quadtree_request.before(update_node_atlas),
                    update_node_atlas,
                    adjust_quadtree.after(update_node_atlas),
                    start_loading_attachment_from_disk.after(update_node_atlas),
                    update_height_under_viewer.after(adjust_quadtree),
                )
                   
            );
        
    }
    fn finish(&self, app: &mut App) {

      
        add_shader(app);

        let render_app = app
            .sub_app_mut(RenderApp)
            .insert_resource(TerrainPipelineConfig {
                attachment_count: self.attachment_count,
            })
            .init_resource::<TerrainComputePipelines>()  
            .init_resource::<SpecializedComputePipelines<TerrainComputePipelines>>()
            .init_resource::<TerrainComponents<GpuNodeAtlas>>()
            .init_resource::<TerrainComponents<TerrainData>>()
            .init_resource::<TerrainViewComponents<GpuQuadtree>>()
            .init_resource::<TerrainViewComponents<TerrainViewData>>()
            .init_resource::<TerrainViewComponents<TerrainViewConfigUniform>>()
            .init_resource::<TerrainViewComponents<CullingBindGroup>>()
            .add_systems(ExtractSchedule,
                (
                    extract_terrain_view_config,
                    initialize_gpu_node_atlas,
                    initialize_gpu_quadtree,
                    initialize_terrain_data.after(initialize_gpu_node_atlas),
                    initialize_terrain_view_data.after(initialize_gpu_quadtree),
                    extract_node_atlas.after(initialize_gpu_node_atlas),
                    extract_quadtree.after(initialize_gpu_quadtree),
                )
                  
            )
            .add_systems(bevy::render::Render,queue_terrain_compute_pipelines.in_set(RenderSet::Queue))
            .add_systems(bevy::render::Render,
                (
                    prepare_quadtree,
                    prepare_node_atlas,
                    prepare_terrain_view_config,
                    prepare_and_queue_terrain_culling_bind_group,
                )
                    .in_set(RenderSet::Prepare),
            );

           
        let compute_node = TerrainComputeNode::from_world(&mut render_app.world);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("terrain_compute", compute_node);
        render_graph.add_node_edge("terrain_compute", CAMERA_DRIVER);

    }
}
