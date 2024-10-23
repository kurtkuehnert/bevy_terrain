use crate::{
    terrain::TerrainComponents,
    terrain_data::{GpuTileAtlas, TileAtlas},
};
use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    prelude::*,
    render::render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
};

pub struct SetTerrainBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainBindGroup<I> {
    type Param = SRes<TerrainComponents<GpuTileAtlas>>;
    type ViewQuery = ();
    type ItemQuery = Read<Handle<TileAtlas>>;

    #[inline]
    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        atlas_handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        tile_atlases: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let gpu_tile_atlas = tile_atlases
            .into_inner()
            .get(&atlas_handle.unwrap().id())
            .unwrap();

        pass.set_bind_group(I, &gpu_tile_atlas.terrain_bind_group, &[]);
        RenderCommandResult::Success
    }
}
