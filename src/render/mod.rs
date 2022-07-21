use crate::{
    render::{
        terrain_data::SetTerrainBindGroup,
        terrain_view_data::{DrawTerrainCommand, SetTerrainViewBindGroup},
    },
    terrain::Terrain,
};
use bevy::{
    pbr::{MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{render_phase::SetItemPipeline, Extract},
};

pub(crate) mod compute_pipelines;
pub(crate) mod culling;
pub(crate) mod layouts;
pub(crate) mod render_pipeline;
pub(crate) mod shaders;
pub(crate) mod terrain_data;
pub(crate) mod terrain_view_data;

/// Configures the default terrain pipeline.
pub struct TerrainPipelineConfig {
    /// The path of the terrain shader.
    pub shader: String,
    /// The number of terrain attachments.
    pub attachment_count: usize,
}

impl Default for TerrainPipelineConfig {
    fn default() -> Self {
        Self {
            shader: "shaders/terrain.wgsl".into(),
            attachment_count: 2,
        }
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
    terrain_query: Extract<Query<(Entity, &GlobalTransform), With<Terrain>>>,
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
