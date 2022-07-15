use crate::debug::change_config;
use crate::{
    attachment_loader::{finish_loading_attachment_from_disk, start_loading_attachment_from_disk},
    debug::{extract_debug, toggle_debug, DebugTerrain},
    node_atlas::update_node_atlas,
    quadtree::{compute_node_updates, traverse_quadtree, update_height_under_viewer, Quadtree},
    render::{
        compute_pipelines::{TerrainComputeNode, TerrainComputePipelines},
        culling::{queue_terrain_culling_bind_group, CullingBindGroup},
        extract_terrain,
        gpu_node_atlas::{
            initialize_gpu_node_atlas, queue_node_atlas_updates, update_gpu_node_atlas,
            GpuNodeAtlas,
        },
        gpu_quadtree::{
            initialize_gpu_quadtree, queue_quadtree_updates, update_gpu_quadtree, GpuQuadtree,
        },
        queue_terrain,
        render_pipeline::{TerrainPipelineConfig, TerrainRenderPipeline},
        terrain_data::{initialize_terrain_data, TerrainData},
        terrain_view_data::{initialize_terrain_view_data, TerrainViewData},
        DrawTerrain,
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
    reflect::TypeUuid,
    render::{
        extract_component::ExtractComponentPlugin, main_graph::node::CAMERA_DRIVER,
        render_graph::RenderGraph, render_phase::AddRenderCommand, render_resource::*, RenderApp,
        RenderStage,
    },
};

pub mod attachment;
pub mod attachment_loader;
pub mod bundles;
pub mod debug;
pub mod node_atlas;
pub mod preprocess;
pub mod quadtree;
pub mod render;
pub mod terrain;
pub mod terrain_view;

const CONFIG_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 907665645684322571);
const TILE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 556563744564564658);
const PARAMETERS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 656456784512075658);
const ATLAS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 124345314345873273);
const TERRAIN_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 234313897973543254);
const DEBUG_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 513467378691355413);

pub const PREPARE_INDIRECT_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 242384313596767307);
pub const UPDATE_QUADTREE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 213403787773215143);
pub const TESSELATION_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 938732132468373352);

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        let mut assets = app.world.resource_mut::<Assets<_>>();
        assets.set_untracked(
            CONFIG_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/config.wgsl")),
        );
        assets.set_untracked(
            TILE_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/tile.wgsl")),
        );
        assets.set_untracked(
            PARAMETERS_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/parameters.wgsl")),
        );
        assets.set_untracked(
            ATLAS_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/atlas.wgsl")),
        );
        assets.set_untracked(
            TERRAIN_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/terrain.wgsl")),
        );
        assets.set_untracked(
            DEBUG_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/debug.wgsl")),
        );
        assets.set_untracked(
            PREPARE_INDIRECT_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/prepare_indirect.wgsl")),
        );
        assets.set_untracked(
            UPDATE_QUADTREE_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/update_quadtree.wgsl")),
        );
        assets.set_untracked(
            TESSELATION_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/tessellation.wgsl")),
        );

        app.add_plugin(ExtractComponentPlugin::<Terrain>::default())
            .add_plugin(ExtractComponentPlugin::<TerrainView>::default())
            .init_resource::<DebugTerrain>()
            .init_resource::<TerrainViewComponents<Quadtree>>()
            .init_resource::<TerrainViewComponents<TerrainViewConfig>>()
            .add_system(toggle_debug)
            .add_system(change_config)
            .add_system(finish_loading_attachment_from_disk.before(update_node_atlas))
            .add_system(traverse_quadtree.before(update_node_atlas))
            .add_system(update_node_atlas)
            .add_system(compute_node_updates.after(update_node_atlas))
            .add_system(update_height_under_viewer.after(compute_node_updates))
            .add_system(start_loading_attachment_from_disk.after(update_node_atlas));

        let render_app = app
            .sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain>()
            .init_resource::<DebugTerrain>()
            .init_resource::<TerrainPipelineConfig>()
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
