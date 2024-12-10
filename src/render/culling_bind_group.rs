use crate::{terrain_data::GpuTileTree, terrain_view::TerrainViewComponents, util::GpuBuffer};
use bevy::render::primitives::Frustum;
use bevy::render::sync_world::MainEntity;
use bevy::utils::HashMap;
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
    half_spaces: [Vec4; 6],
    world_position: Vec3,
}

impl From<&ExtractedView> for CullingUniform {
    fn from(view: &ExtractedView) -> Self {
        let clip_from_world = view.clip_from_view * view.world_from_view.compute_matrix().inverse();

        Self {
            half_spaces: Frustum::from_clip_from_world(&clip_from_world)
                .half_spaces
                .map(|space| space.normal_d()),
            world_position: view.world_from_view.translation(),
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
        let culling_buffer =
            GpuBuffer::<CullingUniform>::create(device, &culling_uniform, BufferUsages::UNIFORM);

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
        extracted_views: Query<(MainEntity, &ExtractedView)>,
        mut culling_bind_groups: ResMut<TerrainViewComponents<CullingBindGroup>>,
    ) {
        // Todo: this is a hack
        let extracted_views = extracted_views
            .into_iter()
            .collect::<HashMap<Entity, &ExtractedView>>();

        for &(terrain, view) in gpu_tile_trees.keys() {
            let extracted_view = *extracted_views.get(&view).unwrap();

            culling_bind_groups.insert(
                (terrain, view),
                CullingBindGroup::new(&device, extracted_view.into()),
            );
        }
    }
}
