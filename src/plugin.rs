use crate::{
    math::sync_terrain_position,
    render::{
        terrain_pass::{
            extract_terrain_phases, prepare_terrain_depth_textures, DepthCopyPipeline, TerrainItem,
            TerrainPass,
        },
        tiling_prepass::{
            queue_tiling_prepass, TerrainTilingPrepassPipelines, TilingPrepass, TilingPrepassItem,
        },
        GpuTerrain, GpuTerrainView,
    },
    shaders::{load_terrain_shaders, InternalShaders},
    terrain::TerrainComponents,
    terrain_data::{GpuTileAtlas, TileAtlas, TileTree},
    terrain_view::TerrainViewComponents,
};
use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    prelude::*,
    render::{
        graph::CameraDriverLabel,
        render_graph::{RenderGraph, RenderGraphApp, ViewNodeRunner},
        render_phase::{sort_phase_system, DrawFunctions, ViewSortedRenderPhases},
        render_resource::*,
        view::{check_visibility, VisibilitySystems},
        Render, RenderApp, RenderSet,
    },
};

/// The plugin for the terrain renderer.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "high_precision")]
        app.add_plugins(crate::big_space::BigSpacePlugin::default());

        app.init_resource::<InternalShaders>()
            .init_resource::<TerrainViewComponents<TileTree>>()
            .add_systems(
                PostUpdate,
                (
                    sync_terrain_position,
                    check_visibility::<With<TileAtlas>>.in_set(VisibilitySystems::CheckVisibility),
                ),
            )
            .add_systems(
                Last,
                (
                    TileTree::compute_requests,
                    TileAtlas::update,
                    TileTree::adjust_to_tile_atlas,
                    #[cfg(feature = "high_precision")]
                    TileTree::generate_surface_approximation,
                    TileTree::update_tile_tree_buffer,
                )
                    .chain(),
            );
        app.sub_app_mut(RenderApp)
            .init_resource::<SpecializedComputePipelines<TerrainTilingPrepassPipelines>>()
            .init_resource::<TerrainComponents<GpuTileAtlas>>()
            .init_resource::<TerrainComponents<GpuTerrain>>()
            .init_resource::<TerrainViewComponents<GpuTerrainView>>()
            .init_resource::<TerrainViewComponents<TilingPrepassItem>>()
            .init_resource::<DrawFunctions<TerrainItem>>()
            .init_resource::<ViewSortedRenderPhases<TerrainItem>>()
            .add_systems(
                ExtractSchedule,
                (
                    extract_terrain_phases,
                    GpuTileAtlas::initialize,
                    GpuTileAtlas::extract.after(GpuTileAtlas::initialize),
                    GpuTerrain::initialize.after(GpuTileAtlas::initialize),
                    GpuTerrain::extract.after(GpuTerrain::initialize),
                    GpuTerrainView::initialize,
                ),
            )
            .add_systems(
                Render,
                (
                    (
                        GpuTileAtlas::prepare,
                        GpuTerrain::prepare,
                        GpuTerrainView::prepare_terrain_view,
                        GpuTerrainView::prepare_indirect,
                        GpuTerrainView::prepare_refine_tiles,
                    )
                        .in_set(RenderSet::Prepare),
                    sort_phase_system::<TerrainItem>.in_set(RenderSet::PhaseSort),
                    prepare_terrain_depth_textures.in_set(RenderSet::PrepareResources),
                    queue_tiling_prepass.in_set(RenderSet::Queue),
                    GpuTileAtlas::cleanup
                        .before(World::clear_entities)
                        .in_set(RenderSet::Cleanup),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<TerrainPass>>(Core3d, TerrainPass)
            .add_render_graph_edges(
                Core3d,
                (Node3d::StartMainPass, TerrainPass, Node3d::MainOpaquePass),
            );

        let mut render_graph = app
            .sub_app_mut(RenderApp)
            .world_mut()
            .resource_mut::<RenderGraph>();
        render_graph.add_node(TilingPrepass, TilingPrepass);
        render_graph.add_node_edge(TilingPrepass, CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        load_terrain_shaders(app);

        app.sub_app_mut(RenderApp)
            .init_resource::<TerrainTilingPrepassPipelines>()
            .init_resource::<DepthCopyPipeline>();
    }
}
