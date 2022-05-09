use crate::{Terrain, TerrainComputePipelines};
use bevy::{
    core::cast_slice,
    pbr::MeshUniform,
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice, view::ExtractedView},
};

#[derive(Component)]
pub struct CullingBindGroup {
    pub(crate) value: BindGroup,
}

pub(crate) fn queue_terrain_culling_bind_group(
    mut commands: Commands,
    device: Res<RenderDevice>,
    compute_pipelines: Res<TerrainComputePipelines>,
    terrain_query: Query<(Entity, &MeshUniform), With<Terrain>>,
    view_query: Query<&ExtractedView>,
) {
    let view = view_query.single();
    let view_proj = view.projection * view.transform.compute_matrix().inverse();

    for (entity, mesh_uniform) in terrain_query.iter() {
        let data = [view_proj, mesh_uniform.transform];

        let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            contents: cast_slice(&data),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let cull_bind_group = device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
            layout: &compute_pipelines.cull_data_layout,
        });

        commands.entity(entity).insert(CullingBindGroup {
            value: cull_bind_group,
        });
    }
}
