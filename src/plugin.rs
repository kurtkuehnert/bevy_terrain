use crate::{
    render::{
        queue_tiling_prepass, CullingBindGroup, TerrainData, TerrainViewData, TilingPrepassItem,
        TilingPrepassLabel, TilingPrepassNode, TilingPrepassPipelines,
    },
    shaders::{load_terrain_shaders, InternalShaders},
    terrain::TerrainComponents,
    terrain_data::{GpuTileAtlas, GpuTileTree, TileAtlas, TileTree},
    terrain_view::TerrainViewComponents,
};
use bevy::{
    prelude::*,
    render::{
        graph::CameraDriverLabel,
        render_graph::RenderGraph,
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
                check_visibility::<With<TileAtlas>>.in_set(VisibilitySystems::CheckVisibility),
            )
            .add_systems(
                Last,
                (
                    TileTree::compute_requests,
                    TileAtlas::update,
                    TileTree::adjust_to_tile_atlas,
                    TileTree::approximate_height,
                    #[cfg(feature = "high_precision")]
                    TileTree::generate_surface_approximation,
                )
                    .chain(),
            );

        app.sub_app_mut(RenderApp)
            .init_resource::<TerrainComponents<GpuTileAtlas>>()
            .init_resource::<TerrainComponents<TerrainData>>()
            .init_resource::<TerrainViewComponents<GpuTileTree>>()
            .init_resource::<TerrainViewComponents<TerrainViewData>>()
            .init_resource::<TerrainViewComponents<CullingBindGroup>>()
            .init_resource::<TerrainViewComponents<TilingPrepassItem>>()
            .add_systems(
                ExtractSchedule,
                (
                    GpuTileAtlas::initialize,
                    GpuTileAtlas::extract.after(GpuTileAtlas::initialize),
                    GpuTileTree::initialize,
                    GpuTileTree::extract.after(GpuTileTree::initialize),
                    TerrainData::initialize.after(GpuTileAtlas::initialize),
                    TerrainData::extract.after(TerrainData::initialize),
                    TerrainViewData::initialize.after(GpuTileTree::initialize),
                    TerrainViewData::extract.after(TerrainViewData::initialize),
                ),
            )
            .add_systems(
                Render,
                (
                    (
                        GpuTileTree::prepare,
                        GpuTileAtlas::prepare,
                        TerrainData::prepare,
                        TerrainViewData::prepare,
                        CullingBindGroup::prepare,
                    )
                        .in_set(RenderSet::Prepare),
                    queue_tiling_prepass.in_set(RenderSet::Queue),
                    GpuTileAtlas::cleanup
                        .before(World::clear_entities)
                        .in_set(RenderSet::Cleanup),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        load_terrain_shaders(app);

        let render_app = app
            .sub_app_mut(RenderApp)
            .init_resource::<TilingPrepassPipelines>()
            .init_resource::<SpecializedComputePipelines<TilingPrepassPipelines>>();

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(TilingPrepassLabel, TilingPrepassNode);
        render_graph.add_node_edge(TilingPrepassLabel, CameraDriverLabel);
    }
}
