use crate::{
    render::{terrain_data::TerrainData, terrain_pipeline::TerrainPipelineKey},
    TerrainPipeline,
};
use bevy::{
    core_pipeline::Opaque3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{wireframe::Wireframe, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{
            DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::*,
    },
};

pub mod compute_pipelines;
pub mod culling;
pub mod layouts;
pub mod terrain_data;
pub mod terrain_pipeline;

pub struct SetTerrainDataBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetTerrainDataBindGroup<I> {
    type Param = (
        SRes<RenderAssets<TerrainData>>,
        SQuery<Read<Handle<TerrainData>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (terrain_data, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let handle = terrain_query.get(item).unwrap();
        let gpu_terrain_data = terrain_data.into_inner().get(handle).unwrap();

        pass.set_bind_group(I, &gpu_terrain_data.terrain_data_bind_group, &[]);

        RenderCommandResult::Success
    }
}

pub struct SetPatchListBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetPatchListBindGroup<I> {
    type Param = (
        SRes<RenderAssets<TerrainData>>,
        SQuery<Read<Handle<TerrainData>>>,
    );

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (terrain_data, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let handle = terrain_query.get(item).unwrap();
        let gpu_terrain_data = terrain_data.into_inner().get(handle).unwrap();

        pass.set_bind_group(I, &gpu_terrain_data.patch_list_bind_group, &[]);

        RenderCommandResult::Success
    }
}

pub(crate) struct DrawTerrainCommand;

impl EntityRenderCommand for DrawTerrainCommand {
    type Param = (
        SRes<RenderAssets<TerrainData>>,
        SQuery<Read<Handle<TerrainData>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (terrain_data, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let handle = terrain_query.get(item).unwrap();
        let gpu_terrain_data = terrain_data.into_inner().get(handle).unwrap();

        pass.inner()
            .draw_indirect(&gpu_terrain_data.indirect_buffer, 0);

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

pub(crate) fn extract_terrain(
    mut commands: Commands,
    terrain_query: Query<(Entity, &GlobalTransform), With<Handle<TerrainData>>>,
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
    terrain_pipeline: Res<TerrainPipeline>,
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedPipelines<TerrainPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut view_query: Query<&mut RenderPhase<Opaque3d>>,
    terrain_query: Query<(Entity, Option<&Wireframe>), With<Handle<TerrainData>>>,
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
