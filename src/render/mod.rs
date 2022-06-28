use crate::terrain::TerrainComponents;
use crate::{
    render::render_pipeline::TerrainPipelineKey, terrain::Terrain, DebugTerrain, TerrainData,
    TerrainRenderPipeline, TerrainViewComponents, TerrainViewData,
};
use bevy::{
    core_pipeline::core_3d::Opaque3d,
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        render_phase::{
            DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::*,
    },
};

pub mod compute_pipelines;
pub mod culling;
pub mod gpu_node_atlas;
pub mod gpu_quadtree;
pub mod layouts;
pub mod render_pipeline;
pub mod terrain_data;
pub mod terrain_view_data;

pub struct SetTerrainBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetTerrainBindGroup<I> {
    type Param = SRes<TerrainComponents<TerrainData>>;

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        terrain_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_data.into_inner().get(&item).unwrap();
        pass.set_bind_group(I, &data.terrain_bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct SetTerrainViewBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetTerrainViewBindGroup<I> {
    type Param = SRes<TerrainViewComponents<TerrainViewData>>;

    #[inline]
    fn render<'w>(
        view: Entity,
        terrain: Entity,
        terrain_view_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_view_data
            .into_inner()
            .get(&(terrain, view))
            .unwrap();

        pass.set_bind_group(I, &data.terrain_view_bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub(crate) struct DrawTerrainCommand;

impl EntityRenderCommand for DrawTerrainCommand {
    type Param = SRes<TerrainViewComponents<TerrainViewData>>;

    #[inline]
    fn render<'w>(
        view: Entity,
        terrain: Entity,
        terrain_view_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let terrain_view = terrain_view_data
            .into_inner()
            .get(&(terrain, view))
            .unwrap();

        pass.draw_indirect(&terrain_view.indirect_buffer, 0);
        RenderCommandResult::Success
    }
}

/// The draw function of the terrain. It sets the pipeline and the bind groups and then issues the
/// draw call.
pub(crate) type DrawTerrain = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetTerrainViewBindGroup<1>,
    SetTerrainBindGroup<2>,
    SetMeshBindGroup<3>,
    DrawTerrainCommand,
);

/// Extracts the [`MeshUniform`] data of all terrains.
pub(crate) fn extract_terrain(
    mut commands: Commands,
    terrain_query: Query<(Entity, &GlobalTransform), With<Terrain>>,
) {
    for (entity, transform) in terrain_query.iter() {
        let transform = transform.compute_matrix();

        commands.get_or_spawn(entity).insert(MeshUniform {
            flags: 0,
            transform,
            inverse_transpose_model: transform.inverse().transpose(),
        });
    }
}

/// Queses all terrain entities for rendering via the terrain pipeline.
pub(crate) fn queue_terrain(
    terrain_pipeline: Res<TerrainRenderPipeline>,
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    msaa: Res<Msaa>,
    debug: Res<DebugTerrain>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TerrainRenderPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut view_query: Query<&mut RenderPhase<Opaque3d>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    let draw_function = draw_functions.read().get_id::<DrawTerrain>().unwrap();

    for mut opaque_phase in view_query.iter_mut() {
        for entity in terrain_query.iter() {
            let key = TerrainPipelineKey::from_msaa_samples(msaa.samples)
                | TerrainPipelineKey::from_debug(&debug);

            let pipeline = pipelines.specialize(&mut pipeline_cache, &terrain_pipeline, key);

            opaque_phase.add(Opaque3d {
                entity,
                pipeline,
                draw_function,
                distance: f32::MIN, // draw terrain first
            });
        }
    }
}
