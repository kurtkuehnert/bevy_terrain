use crate::{
    render::{
        render_pipeline::TerrainPipelineKey,
        terrain_data::SetTerrainBindGroup,
        terrain_view_data::{DrawTerrainCommand, SetTerrainViewBindGroup},
    },
    terrain::Terrain,
    DebugTerrain, TerrainRenderPipeline,
};
use bevy::{
    core_pipeline::core_3d::Opaque3d,
    pbr::{MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        render_phase::{DrawFunctions, EntityRenderCommand, RenderPhase, SetItemPipeline},
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
