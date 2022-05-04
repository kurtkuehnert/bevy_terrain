use crate::{
    render::{bind_groups::TerrainBindGroups, render_pipeline::TerrainPipelineKey},
    TerrainConfig, TerrainRenderPipeline,
};
use bevy::{
    core_pipeline::Opaque3d,
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{wireframe::Wireframe, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        render_phase::{
            DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::*,
    },
    utils::HashMap,
};

pub mod bind_groups;
pub mod compute_pipelines;
pub mod culling;
pub mod gpu_node_atlas;
pub mod gpu_quadtree;
pub mod height_attachment;
pub mod layouts;
pub mod render_pipeline;
pub mod resources;

pub type PersistentComponent<A> = HashMap<Entity, A>;

#[derive(Component)]
pub(crate) struct InitTerrain;

pub struct SetTerrainDataBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetTerrainDataBindGroup<I> {
    type Param = SRes<PersistentComponent<TerrainBindGroups>>;

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        terrain_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_groups = terrain_bind_groups.into_inner().get(&item).unwrap();
        pass.set_bind_group(I, &bind_groups.terrain_data_bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct SetPatchListBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetPatchListBindGroup<I> {
    type Param = SRes<PersistentComponent<TerrainBindGroups>>;

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        terrain_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_groups = terrain_bind_groups.into_inner().get(&item).unwrap();
        pass.set_bind_group(I, &bind_groups.patch_list_bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub(crate) struct DrawTerrainCommand;

impl EntityRenderCommand for DrawTerrainCommand {
    type Param = SRes<PersistentComponent<TerrainBindGroups>>;

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        terrain_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_groups = terrain_bind_groups.into_inner().get(&item).unwrap();
        pass.draw_indirect(&bind_groups.indirect_buffer, 0);
        RenderCommandResult::Success
    }
}

/// The draw function of the terrain. It sets the pipeline and the bind groups and then issues the
/// draw call.
pub(crate) type DrawTerrain = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetTerrainDataBindGroup<2>,
    SetPatchListBindGroup<3>,
    DrawTerrainCommand,
);

/// Runs in extract.
pub(crate) fn notify_init_terrain(
    mut commands: Commands,
    terrain_query: Query<Entity, Changed<TerrainConfig>>,
) {
    for entity in terrain_query.iter() {
        commands.get_or_spawn(entity).insert(InitTerrain);
    }
}

pub(crate) fn extract_terrain(
    mut commands: Commands,
    terrain_query: Query<(Entity, &GlobalTransform), With<TerrainConfig>>,
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
    mut pipelines: ResMut<SpecializedRenderPipelines<TerrainRenderPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut view_query: Query<&mut RenderPhase<Opaque3d>>,
    terrain_query: Query<(Entity, Option<&Wireframe>), With<TerrainConfig>>,
) {
    let draw_function = draw_functions.read().get_id::<DrawTerrain>().unwrap();

    for mut opaque_phase in view_query.iter_mut() {
        for (entity, wireframe) in terrain_query.iter() {
            let key = TerrainPipelineKey::from_msaa_samples(msaa.samples)
                | TerrainPipelineKey::from_wireframe(wireframe.is_some());

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
