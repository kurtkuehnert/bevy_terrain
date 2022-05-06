use crate::attachments::{finish_loading_attachment_from_disk, start_loading_attachment_from_disk};
use crate::node_atlas::LoadNodeEvent;
use crate::{
    config::TerrainConfig,
    node_atlas::update_nodes,
    persistent_component::{PersistentComponentPlugin, PersistentComponents},
    quadtree::traverse_quadtree,
    render::{
        bind_groups::{init_terrain_bind_groups, TerrainBindGroups},
        compute_pipelines::{TerrainComputeNode, TerrainComputePipelines},
        culling::queue_terrain_culling_bind_group,
        extract_terrain,
        gpu_node_atlas::{queue_node_attachment_updates, GpuNodeAtlas},
        gpu_quadtree::{queue_quadtree_updates, GpuQuadtree},
        notify_init_terrain, queue_terrain,
        render_pipeline::TerrainRenderPipeline,
        resources::initialize_terrain_resources,
        DrawTerrain,
    },
};
use bevy::{
    core_pipeline::{node::MAIN_PASS_DEPENDENCIES, Opaque3d},
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_component::ExtractComponentPlugin,
        render_graph::RenderGraph,
        render_phase::AddRenderCommand,
        render_resource::{SpecializedComputePipelines, SpecializedRenderPipelines},
        RenderApp, RenderStage,
    },
};

pub mod attachments;
pub mod bundles;
pub mod config;
pub mod node_atlas;
pub mod persistent_component;
pub mod preprocess;
pub mod quadtree;
pub mod render;
pub mod viewer;

const CONFIG_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 907665645684322571);
const NODE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 456563743231345678);
const PATCH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 556563744564564658);
const PARAMETERS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 656456784512075658);

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
            NODE_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/node.wgsl")),
        );
        assets.set_untracked(
            PATCH_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/patch.wgsl")),
        );
        assets.set_untracked(
            PARAMETERS_HANDLE,
            Shader::from_wgsl(include_str!("render/shaders/parameters.wgsl")),
        );

        app.add_plugin(ExtractComponentPlugin::<TerrainConfig>::default())
            .add_plugin(PersistentComponentPlugin::<GpuQuadtree>::default())
            .add_plugin(PersistentComponentPlugin::<GpuNodeAtlas>::default())
            .add_event::<LoadNodeEvent>()
            .add_system(traverse_quadtree.before(update_nodes))
            .add_system(update_nodes)
            .add_system(start_loading_attachment_from_disk.after(update_nodes))
            .add_system(finish_loading_attachment_from_disk);

        let render_app = app
            .sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain>()
            .init_resource::<TerrainComputePipelines>()
            .init_resource::<SpecializedComputePipelines<TerrainComputePipelines>>()
            .init_resource::<TerrainRenderPipeline>()
            .init_resource::<SpecializedRenderPipelines<TerrainRenderPipeline>>()
            .init_resource::<PersistentComponents<TerrainBindGroups>>()
            .add_system_to_stage(RenderStage::Extract, notify_init_terrain)
            .add_system_to_stage(RenderStage::Extract, extract_terrain)
            .add_system_to_stage(RenderStage::Prepare, initialize_terrain_resources)
            .add_system_to_stage(RenderStage::Queue, init_terrain_bind_groups)
            .add_system_to_stage(RenderStage::Queue, queue_terrain)
            .add_system_to_stage(RenderStage::Queue, queue_quadtree_updates)
            .add_system_to_stage(RenderStage::Queue, queue_terrain_culling_bind_group)
            .add_system_to_stage(RenderStage::Queue, queue_node_attachment_updates);

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
