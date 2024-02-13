use crate::{
    terrain::Terrain,
    terrain_view::{TerrainView, TerrainViewComponents},
    util::StaticBuffer,
};
use bevy::{
    prelude::*,
    render::{
        render_resource::{binding_types::*, *},
        renderer::RenderDevice,
        view::ExtractedView,
    },
};
use std::ops::Deref;

pub(crate) fn create_culling_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::single(
            ShaderStages::COMPUTE,
            uniform_buffer::<CullingUniform>(false), // culling data
        ),
    )
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

#[derive(Default, ShaderType)]
pub struct CullingUniform {
    world_position: Vec3,
    view_proj: Mat4,
    model: Mat4,
    planes: [Vec4; 5],
}

impl From<&ExtractedView> for CullingUniform {
    fn from(view: &ExtractedView) -> Self {
        let view_proj = view.projection * view.transform.compute_matrix().inverse();
        let world_position = view.transform.translation();
        let planes = planes(&view_proj);

        Self {
            world_position,
            view_proj,
            model: default(),
            planes,
        }
    }
}

#[derive(Component)]
pub struct CullingBindGroup(BindGroup);

impl Deref for CullingBindGroup {
    type Target = BindGroup;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CullingBindGroup {
    fn new(device: &RenderDevice, culling_uniform: CullingUniform) -> Self {
        let culling_buffer = StaticBuffer::<CullingUniform>::create(
            None,
            device,
            &culling_uniform,
            BufferUsages::UNIFORM,
        );

        let bind_group = device.create_bind_group(
            None,
            &create_culling_layout(device),
            &BindGroupEntries::single(&culling_buffer),
        );

        Self(bind_group)
    }

    pub(crate) fn prepare(
        device: Res<RenderDevice>,
        mut culling_bind_groups: ResMut<TerrainViewComponents<CullingBindGroup>>,
        terrain_query: Query<Entity, With<Terrain>>,
        view_query: Query<(Entity, &ExtractedView), With<TerrainView>>,
    ) {
        for (view, extracted_view) in view_query.iter() {
            // todo: save per view not per terrain

            for terrain in terrain_query.iter() {
                let culling_bind_group = CullingBindGroup::new(&device, extracted_view.into());

                culling_bind_groups.insert((terrain, view), culling_bind_group);
            }
        }
    }
}
