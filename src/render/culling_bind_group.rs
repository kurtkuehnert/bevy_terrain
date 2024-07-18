use crate::{terrain_data::GpuTileTree, terrain_view::TerrainViewComponents, util::StaticBuffer};
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

#[derive(Default, ShaderType)]
pub struct CullingUniform {
    world_position: Vec3,
    view_proj: Mat4,
    planes: [Vec4; 5],
}

impl From<&ExtractedView> for CullingUniform {
    fn from(view: &ExtractedView) -> Self {
        Self {
            world_position: view.world_from_view.translation(),
            view_proj: view.world_from_view.compute_matrix().inverse(),
            planes: default(),
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
        gpu_tile_trees: Res<TerrainViewComponents<GpuTileTree>>,
        extracted_views: Query<&ExtractedView>,
        mut culling_bind_groups: ResMut<TerrainViewComponents<CullingBindGroup>>,
    ) {
        for &(terrain, view) in gpu_tile_trees.keys() {
            let extracted_view = extracted_views.get(view).unwrap();

            culling_bind_groups.insert(
                (terrain, view),
                CullingBindGroup::new(&device, extracted_view.into()),
            );
        }
    }
}
