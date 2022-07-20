use crate::data_structures::gpu_node_atlas::{
    initialize_gpu_node_atlas, queue_node_atlas_updates, update_gpu_node_atlas, GpuNodeAtlas,
};
use crate::render::shaders::add_shader;
use crate::{
    attachment_loader::{finish_loading_attachment_from_disk, start_loading_attachment_from_disk},
    data_structures::{
        gpu_quadtree::{
            initialize_gpu_quadtree, queue_quadtree_updates, update_gpu_quadtree, GpuQuadtree,
        },
        node_atlas::update_node_atlas,
        quadtree::{
            adjust_quadtree, compute_quadtree_request, update_height_under_viewer, Quadtree,
        },
    },
    debug::{change_config, extract_debug, toggle_debug, DebugTerrain},
    render::{
        compute_pipelines::{TerrainComputeNode, TerrainComputePipelines},
        culling::{queue_terrain_culling_bind_group, CullingBindGroup},
        extract_terrain,
        render_pipeline::{queue_terrain, TerrainRenderPipeline},
        terrain_data::{initialize_terrain_data, TerrainData},
        terrain_view_data::{initialize_terrain_view_data, TerrainViewData},
        DrawTerrain, TerrainPipelineConfig,
    },
    terrain::{Terrain, TerrainComponents, TerrainConfig},
    terrain_view::{
        extract_terrain_view_config, queue_terrain_view_config, TerrainView, TerrainViewComponents,
        TerrainViewConfig,
    },
};
use bevy::{
    core_pipeline::core_3d::Opaque3d,
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin, main_graph::node::CAMERA_DRIVER,
        render_graph::RenderGraph, render_phase::AddRenderCommand, render_resource::*, RenderApp,
        RenderStage,
    },
};

mod attachment_loader;
mod bundles;
mod data_structures;
mod debug;
pub mod preprocess;
mod render;
mod terrain;
mod terrain_view;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        attachment_loader::AttachmentFromDiskLoader,
        bundles::TerrainBundle,
        data_structures::quadtree::Quadtree,
        preprocess::prelude,
        render::TerrainPipelineConfig,
        terrain::{Terrain, TerrainConfig},
        terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
        TerrainPlugin,
    };
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        add_shader(app);

        app.add_plugin(ExtractComponentPlugin::<Terrain>::default())
            .add_plugin(ExtractComponentPlugin::<TerrainView>::default())
            .init_resource::<DebugTerrain>()
            .init_resource::<TerrainViewComponents<Quadtree>>()
            .init_resource::<TerrainViewComponents<TerrainViewConfig>>()
            .add_system(toggle_debug)
            .add_system(change_config)
            .add_system_to_stage(
                CoreStage::Last,
                finish_loading_attachment_from_disk.before(update_node_atlas),
            )
            .add_system_to_stage(
                CoreStage::Last,
                compute_quadtree_request.before(update_node_atlas),
            )
            .add_system_to_stage(CoreStage::Last, update_node_atlas)
            .add_system_to_stage(CoreStage::Last, adjust_quadtree.after(update_node_atlas))
            .add_system_to_stage(
                CoreStage::Last,
                start_loading_attachment_from_disk.after(update_node_atlas),
            )
            .add_system_to_stage(
                CoreStage::Last,
                update_height_under_viewer.after(adjust_quadtree),
            );

        let config = app
            .world
            .remove_resource::<TerrainPipelineConfig>()
            .unwrap_or(default());

        let render_app = app
            .sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain>()
            .insert_resource(config)
            .init_resource::<DebugTerrain>()
            .init_resource::<TerrainRenderPipeline>()
            .init_resource::<SpecializedRenderPipelines<TerrainRenderPipeline>>()
            .init_resource::<TerrainComputePipelines>()
            .init_resource::<SpecializedComputePipelines<TerrainComputePipelines>>()
            .init_resource::<TerrainComponents<GpuNodeAtlas>>()
            .init_resource::<TerrainComponents<TerrainData>>()
            .init_resource::<TerrainViewComponents<GpuQuadtree>>()
            .init_resource::<TerrainViewComponents<TerrainViewData>>()
            .init_resource::<TerrainViewComponents<TerrainViewConfig>>()
            .init_resource::<TerrainViewComponents<CullingBindGroup>>()
            .add_system_to_stage(RenderStage::Extract, extract_terrain)
            .add_system_to_stage(RenderStage::Extract, extract_terrain_view_config)
            .add_system_to_stage(RenderStage::Extract, extract_debug)
            .add_system_to_stage(RenderStage::Extract, initialize_gpu_node_atlas)
            .add_system_to_stage(RenderStage::Extract, initialize_gpu_quadtree)
            .add_system_to_stage(
                RenderStage::Extract,
                initialize_terrain_data.after(initialize_gpu_node_atlas),
            )
            .add_system_to_stage(
                RenderStage::Extract,
                initialize_terrain_view_data.after(initialize_gpu_quadtree),
            )
            .add_system_to_stage(
                RenderStage::Extract,
                update_gpu_node_atlas.after(initialize_gpu_node_atlas),
            )
            .add_system_to_stage(
                RenderStage::Extract,
                update_gpu_quadtree.after(initialize_gpu_quadtree),
            )
            .add_system_to_stage(RenderStage::Queue, queue_terrain)
            .add_system_to_stage(RenderStage::Queue, queue_quadtree_updates)
            .add_system_to_stage(RenderStage::Queue, queue_node_atlas_updates)
            .add_system_to_stage(RenderStage::Queue, queue_terrain_culling_bind_group)
            .add_system_to_stage(RenderStage::Queue, queue_terrain_view_config);

        let compute_node = TerrainComputeNode::from_world(&mut render_app.world);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("terrain_compute", compute_node);

        render_graph
            .add_node_edge("terrain_compute", CAMERA_DRIVER)
            .unwrap();
    }
}
