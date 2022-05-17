use crate::debug::{extract_debug, toggle_debug_system, DebugTerrain};
use crate::{
    attachment_loader::{finish_loading_attachment_from_disk, start_loading_attachment_from_disk},
    config::TerrainConfig,
    node_atlas::{update_nodes, LoadNodeEvent},
    quadtree::traverse_quadtree,
    render::{
        compute_data::{initialize_terrain_compute_data, TerrainComputeData},
        compute_pipelines::{TerrainComputeNode, TerrainComputePipelines},
        culling::queue_terrain_culling_bind_group,
        extract_terrain,
        gpu_node_atlas::{
            initialize_gpu_node_atlas, queue_node_atlas_updates, update_gpu_node_atlas,
            GpuNodeAtlas,
        },
        gpu_quadtree::{
            initialize_gpu_quadtree, queue_quadtree_updates, update_gpu_quadtree, GpuQuadtree,
        },
        queue_terrain,
        render_data::{initialize_terrain_render_data, TerrainRenderData},
        render_pipeline::TerrainRenderPipeline,
        resources::initialize_terrain_resources,
        DrawTerrain, PersistentComponents,
    },
};
use bevy::{
    core_pipeline::{node::MAIN_PASS_DEPENDENCIES, Opaque3d},
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_component::{ExtractComponent, ExtractComponentPlugin},
        render_graph::RenderGraph,
        render_phase::AddRenderCommand,
        render_resource::{SpecializedComputePipelines, SpecializedRenderPipelines},
        RenderApp, RenderStage,
    },
};

pub mod attachment;
pub mod attachment_loader;
pub mod bundles;
pub mod config;
pub mod debug;
pub mod node_atlas;
pub mod preprocess;
pub mod quadtree;
pub mod render;
pub mod viewer;

const CONFIG_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 907665645684322571);
const PATCH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 556563744564564658);
const PARAMETERS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 656456784512075658);
const ATLAS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 124345314345873273);

#[derive(Clone, Copy, Component)]
pub struct Terrain;

impl ExtractComponent for Terrain {
    type Query = Read<Terrain>;
    type Filter = ();

    #[inline]
    fn extract_component(_item: QueryItem<Self::Query>) -> Self {
        Self
    }
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // register_inspectable(app);

        let mut assets = app.world.resource_mut::<Assets<_>>();
        assets.set_untracked(
            CONFIG_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/config.wgsl")),
        );
        assets.set_untracked(
            PATCH_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/patch.wgsl")),
        );
        assets.set_untracked(
            PARAMETERS_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/parameters.wgsl")),
        );
        assets.set_untracked(
            ATLAS_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/atlas.wgsl")),
        );

        app.add_plugin(ExtractComponentPlugin::<Terrain>::default())
            .add_plugin(ExtractComponentPlugin::<TerrainConfig>::default())
            .init_resource::<DebugTerrain>()
            .add_event::<LoadNodeEvent>()
            .add_system(traverse_quadtree.before(update_nodes))
            .add_system(update_nodes)
            .add_system(start_loading_attachment_from_disk.after(update_nodes))
            .add_system(finish_loading_attachment_from_disk)
            .add_system(toggle_debug_system);

        let render_app = app
            .sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain>()
            .init_resource::<TerrainComputePipelines>()
            .init_resource::<SpecializedComputePipelines<TerrainComputePipelines>>()
            .init_resource::<TerrainRenderPipeline>()
            .init_resource::<SpecializedRenderPipelines<TerrainRenderPipeline>>()
            .init_resource::<PersistentComponents<TerrainComputeData>>()
            .init_resource::<PersistentComponents<GpuQuadtree>>()
            .init_resource::<PersistentComponents<GpuNodeAtlas>>()
            .init_resource::<PersistentComponents<TerrainRenderData>>()
            .add_system_to_stage(RenderStage::Extract, extract_terrain)
            .add_system_to_stage(RenderStage::Extract, extract_debug)
            .add_system_to_stage(RenderStage::Extract, update_gpu_quadtree)
            .add_system_to_stage(RenderStage::Extract, update_gpu_node_atlas)
            .add_system_to_stage(RenderStage::Prepare, initialize_terrain_resources)
            .add_system_to_stage(RenderStage::Prepare, initialize_gpu_quadtree)
            .add_system_to_stage(RenderStage::Prepare, initialize_gpu_node_atlas)
            .add_system_to_stage(RenderStage::Queue, initialize_terrain_compute_data)
            .add_system_to_stage(RenderStage::Queue, initialize_terrain_render_data)
            .add_system_to_stage(
                RenderStage::Queue,
                queue_terrain.after(initialize_terrain_render_data),
            )
            .add_system_to_stage(RenderStage::Queue, queue_quadtree_updates)
            .add_system_to_stage(RenderStage::Queue, queue_node_atlas_updates)
            .add_system_to_stage(RenderStage::Queue, queue_terrain_culling_bind_group);

        let compute_node = TerrainComputeNode::from_world(&mut render_app.world);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("terrain_compute", compute_node);
        render_graph
            .add_node_edge("terrain_compute", MAIN_PASS_DEPENDENCIES)
            .unwrap();
    }
}

// fn register_inspectable(app: &mut App) {
//     app.register_inspectable::<ViewDistance>()
//         .register_inspectable::<TerrainDebugInfo>();
// }
