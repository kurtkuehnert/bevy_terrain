use crate::{
    formats::TDFPlugin,
    render::{
        compute_pipelines::{TerrainComputeNode, TerrainComputePipelines},
        culling_bind_group::CullingBindGroup,
        shaders::load_terrain_shaders,
        terrain_bind_group::TerrainData,
        terrain_view_bind_group::TerrainViewData,
    },
    terrain::{Terrain, TerrainComponents},
    terrain_data::{
        gpu_node_atlas::GpuNodeAtlas, gpu_quadtree::GpuQuadtree, node_atlas::NodeAtlas,
        quadtree::Quadtree,
    },
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
};
use bevy::{
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin, main_graph::node::CAMERA_DRIVER,
        render_graph::RenderGraph, render_resource::*, Render, RenderApp, RenderSet,
    },
};

/// The plugin for the terrain renderer.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TDFPlugin,
            ExtractComponentPlugin::<Terrain>::default(),
            ExtractComponentPlugin::<TerrainView>::default(),
        ))
        .init_resource::<TerrainViewComponents<Quadtree>>()
        .init_resource::<TerrainViewComponents<TerrainViewConfig>>()
        .add_systems(
            Last,
            (
                Quadtree::compute_requests,
                NodeAtlas::update,
                Quadtree::adjust_to_node_atlas,
                Quadtree::approximate_height,
            )
                .chain(),
        );

        app.sub_app_mut(RenderApp)
            .init_resource::<TerrainComponents<GpuNodeAtlas>>()
            .init_resource::<TerrainComponents<TerrainData>>()
            .init_resource::<TerrainViewComponents<GpuQuadtree>>()
            .init_resource::<TerrainViewComponents<TerrainViewData>>()
            .init_resource::<TerrainViewComponents<CullingBindGroup>>()
            .add_systems(
                ExtractSchedule,
                (
                    GpuNodeAtlas::initialize,
                    GpuQuadtree::initialize,
                    TerrainData::initialize.after(GpuNodeAtlas::initialize),
                    TerrainViewData::initialize.after(GpuQuadtree::initialize),
                    GpuNodeAtlas::extract.after(GpuNodeAtlas::initialize),
                    GpuQuadtree::extract.after(GpuQuadtree::initialize),
                    TerrainViewData::extract.after(TerrainViewData::initialize),
                ),
            )
            .add_systems(
                Render,
                (
                    (
                        GpuQuadtree::prepare,
                        GpuNodeAtlas::prepare,
                        TerrainViewData::prepare,
                        CullingBindGroup::prepare,
                    )
                        .in_set(RenderSet::Prepare),
                    TerrainComputePipelines::queue.in_set(RenderSet::Queue),
                    GpuNodeAtlas::cleanup
                        .before(World::clear_entities)
                        .in_set(RenderSet::Cleanup),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        load_terrain_shaders(app);

        let render_app = app
            .sub_app_mut(RenderApp)
            .init_resource::<TerrainComputePipelines>()
            .init_resource::<SpecializedComputePipelines<TerrainComputePipelines>>();

        let compute_node = TerrainComputeNode::from_world(&mut render_app.world);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("terrain_compute", compute_node);
        render_graph.add_node_edge("terrain_compute", CAMERA_DRIVER);
    }
}
