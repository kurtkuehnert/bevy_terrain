#[cfg(feature = "high_precision")]
use crate::big_space::BigSpacePlugin;

use crate::{
    math::{generate_terrain_model_approximation, TerrainModelApproximation},
    render::{
        culling_bind_group::CullingBindGroup,
        terrain_bind_group::TerrainData,
        terrain_view_bind_group::TerrainViewData,
        tiling_prepass::{
            queue_tiling_prepass, TilingPrepassItem, TilingPrepassLabel, TilingPrepassNode,
            TilingPrepassPipelines,
        },
    },
    shaders::{load_terrain_shaders, InternalShaders},
    terrain::{Terrain, TerrainComponents, TerrainConfig},
    terrain_data::{
        gpu_node_atlas::GpuNodeAtlas, gpu_quadtree::GpuQuadtree, node_atlas::NodeAtlas,
        quadtree::Quadtree,
    },
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
};
use bevy::{
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin,
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
        app.add_plugins((
            #[cfg(feature = "high_precision")]
            BigSpacePlugin::default(),
            ExtractComponentPlugin::<Terrain>::default(),
            ExtractComponentPlugin::<TerrainView>::default(),
            ExtractComponentPlugin::<TerrainConfig>::default(),
        ))
        .init_resource::<InternalShaders>()
        .init_resource::<TerrainViewComponents<Quadtree>>()
        .init_resource::<TerrainViewComponents<TerrainViewConfig>>()
        .init_resource::<TerrainViewComponents<TerrainModelApproximation>>()
        .add_systems(
            PostUpdate,
            check_visibility::<With<Terrain>>.in_set(VisibilitySystems::CheckVisibility),
        )
        .add_systems(
            Last,
            (
                Quadtree::compute_requests,
                NodeAtlas::update,
                Quadtree::adjust_to_node_atlas,
                Quadtree::approximate_height,
                generate_terrain_model_approximation,
            ),
        );

        app.sub_app_mut(RenderApp)
            .init_resource::<TerrainComponents<GpuNodeAtlas>>()
            .init_resource::<TerrainComponents<TerrainData>>()
            .init_resource::<TerrainViewComponents<GpuQuadtree>>()
            .init_resource::<TerrainViewComponents<TerrainViewData>>()
            .init_resource::<TerrainViewComponents<CullingBindGroup>>()
            .init_resource::<TerrainViewComponents<TilingPrepassItem>>()
            .add_systems(
                ExtractSchedule,
                (
                    GpuNodeAtlas::initialize,
                    GpuQuadtree::initialize,
                    TerrainData::initialize.after(GpuNodeAtlas::initialize),
                    TerrainViewData::initialize.after(GpuQuadtree::initialize),
                    GpuNodeAtlas::extract.after(GpuNodeAtlas::initialize),
                    GpuQuadtree::extract.after(GpuQuadtree::initialize),
                    TerrainData::extract.after(TerrainData::initialize),
                    TerrainViewData::extract.after(TerrainViewData::initialize),
                ),
            )
            .add_systems(
                Render,
                (
                    (
                        GpuQuadtree::prepare,
                        GpuNodeAtlas::prepare,
                        TerrainData::prepare,
                        TerrainViewData::prepare,
                        CullingBindGroup::prepare,
                    )
                        .in_set(RenderSet::Prepare),
                    queue_tiling_prepass.in_set(RenderSet::Queue),
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
            .init_resource::<TilingPrepassPipelines>()
            .init_resource::<SpecializedComputePipelines<TilingPrepassPipelines>>();

        let prepass_node = TilingPrepassNode::from_world(render_app.world_mut());
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(TilingPrepassLabel, prepass_node);
        render_graph.add_node_edge(TilingPrepassLabel, CameraDriverLabel);
    }
}
