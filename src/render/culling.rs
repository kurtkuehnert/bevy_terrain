use crate::{terrain::Terrain, TerrainComputePipelines, TerrainView, TerrainViewComponents};
use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice, view::ExtractedView},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, ShaderType)]
pub struct CullingData {
    pub(crate) world_position: Vec4,
    pub(crate) view_proj: Mat4,
    pub(crate) model: Mat4,
    pub(crate) planes: [Vec4; 5],
}

#[derive(Component)]
pub struct CullingBindGroup {
    pub(crate) value: BindGroup,
}

pub fn planes(view_projection: &Mat4) -> [Vec4; 5] {
    let row3 = view_projection.row(3);
    let mut planes = [default(); 5];
    for (i, plane) in planes.iter_mut().enumerate() {
        let row = view_projection.row(i / 2);
        *plane = if (i & 1) == 0 && i != 4 {
            row3 + row
        } else {
            row3 - row
        };
    }

    planes
}

pub(crate) fn prepare_and_queue_terrain_culling_bind_group(
    device: Res<RenderDevice>,
    compute_pipelines: Res<TerrainComputePipelines>,
    mut culling_bind_groups: ResMut<TerrainViewComponents<CullingBindGroup>>,
    terrain_query: Query<Entity, With<Terrain>>,
    view_query: Query<(Entity, &ExtractedView), With<TerrainView>>,
) {
    for (view, extracted_view) in view_query.iter() {
        let view_proj =
            extracted_view.projection * extracted_view.transform.compute_matrix().inverse();

        let planes = planes(&view_proj);

        for terrain in terrain_query.iter() {
            let culling_data = CullingData {
                world_position: extracted_view.transform.translation().xyzx(),
                view_proj,
                model: default(),
                planes,
            };

            let mut buffer = encase::UniformBuffer::new(Vec::new());
            buffer.write(&culling_data).unwrap();

            let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
                label: None,
                contents: &buffer.into_inner(),
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

            culling_bind_groups.insert(
                (terrain, view),
                CullingBindGroup {
                    value: cull_bind_group,
                },
            );
        }
    }
}
