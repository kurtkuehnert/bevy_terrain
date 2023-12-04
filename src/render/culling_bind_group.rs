use crate::{
    terrain::Terrain,
    terrain_view::{TerrainView, TerrainViewComponents},
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets, render_resource::*, renderer::RenderDevice,
        texture::FallbackImage, view::ExtractedView,
    },
};

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

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, AsBindGroup)]
pub struct CullingData {
    #[uniform(0)]
    world_position: Vec3,
    #[uniform(0)]
    view_proj: Mat4,
    #[uniform(0)]
    model: Mat4,
    #[uniform(0)]
    planes: [Vec4; 5],
}

impl From<&ExtractedView> for CullingData {
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
pub struct CullingBindGroup(PreparedBindGroup<()>);

impl CullingBindGroup {
    fn new(
        extracted_view: &ExtractedView,
        device: &RenderDevice,
        images: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Self {
        let layout = Self::layout(&device);

        let culling_data = CullingData::from(extracted_view);

        let bind_group = culling_data
            .as_bind_group(&layout, &device, &images, &fallback_image)
            .ok()
            .unwrap();

        Self(bind_group)
    }

    pub(crate) fn bind_group(&self) -> &BindGroup {
        &self.0.bind_group
    }

    pub(crate) fn layout(device: &RenderDevice) -> BindGroupLayout {
        CullingData::bind_group_layout(device)
    }

    pub(crate) fn prepare(
        device: Res<RenderDevice>,
        images: Res<RenderAssets<Image>>,
        fallback_image: Res<FallbackImage>,
        mut culling_bind_groups: ResMut<TerrainViewComponents<CullingBindGroup>>,
        terrain_query: Query<Entity, With<Terrain>>,
        view_query: Query<(Entity, &ExtractedView), With<TerrainView>>,
    ) {
        for (view, extracted_view) in view_query.iter() {
            // todo: save per view not per terrain

            for terrain in terrain_query.iter() {
                let culling_bind_group =
                    CullingBindGroup::new(extracted_view, &device, &images, &fallback_image);

                culling_bind_groups.insert((terrain, view), culling_bind_group);
            }
        }
    }
}
