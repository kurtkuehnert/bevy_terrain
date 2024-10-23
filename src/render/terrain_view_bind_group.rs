use crate::prelude::TileAtlas;
use crate::{
    math::{TileCoordinate, ViewCoordinate},
    terrain_data::{GpuTileTree, TileTree},
    terrain_view::TerrainViewComponents,
    util::StaticBuffer,
};
use bevy::ecs::system::lifetimeless::Read;
use bevy::math::Affine3A;
use bevy::pbr::{MeshTransforms, MeshUniform, PreviousGlobalTransform};
use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    prelude::*,
    render::{
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{binding_types::*, *},
        renderer::{RenderDevice, RenderQueue},
        Extract,
    },
};
use std::sync::{Arc, Mutex};
use wgpu::util::DownloadBuffer;

pub(crate) fn create_prepare_indirect_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::single(
            ShaderStages::COMPUTE,
            storage_buffer::<Indirect>(false), // indirect_buffer
        ),
    )
}

pub(crate) fn create_refine_tiles_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                storage_buffer_read_only::<TerrainView>(false), // terrain_view
                storage_buffer_sized(false, None),              // approximate_height
                storage_buffer_read_only_sized(false, None),    // tile_tree
                storage_buffer_sized(false, None),              // final_tiles
                storage_buffer_sized(false, None),              // temporary_tiles
                storage_buffer::<Parameters>(false),            // parameters
            ),
        ),
    )
}

pub(crate) fn create_terrain_view_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                storage_buffer_read_only::<TerrainView>(false), // terrain_view
                storage_buffer_sized(false, None),              // approximate_height
                storage_buffer_read_only_sized(false, None),    // tile_tree
                storage_buffer_read_only_sized(false, None),    // geometry_tiles
            ),
        ),
    )
}

#[derive(Default, ShaderType)]
pub(crate) struct Indirect {
    x_or_vertex_count: u32,
    y_or_instance_count: u32,
    z_or_base_vertex: u32,
    base_instance: u32,
}

#[derive(Default, ShaderType)]
struct Parameters {
    tile_count: u32,
    counter: i32,
    child_index: i32,
    final_index: i32,
}

#[derive(Default, ShaderType)]
struct TerrainView {
    tree_size: u32,
    geometry_tile_count: u32,
    refinement_count: u32,
    grid_size: f32,
    vertices_per_row: u32,
    vertices_per_tile: u32,
    morph_distance: f32,
    blend_distance: f32,
    load_distance: f32,
    subdivision_distance: f32,
    morph_range: f32,
    blend_range: f32,
    precision_threshold_distance: f32,
    view_face: u32,
    view_lod: u32,
    view_coordinates: [ViewCoordinate; 6],
    #[cfg(feature = "high_precision")]
    surface_approximation: [crate::math::SurfaceApproximation; 6],
}

impl TerrainView {
    fn from_tile_tree(tile_tree: &TileTree) -> Self {
        TerrainView {
            tree_size: tile_tree.tree_size,
            geometry_tile_count: tile_tree.geometry_tile_count,
            refinement_count: tile_tree.refinement_count,
            grid_size: tile_tree.grid_size as f32,
            vertices_per_row: 2 * (tile_tree.grid_size + 2),
            vertices_per_tile: 2 * tile_tree.grid_size * (tile_tree.grid_size + 2),
            morph_distance: tile_tree.morph_distance as f32,
            blend_distance: tile_tree.blend_distance as f32,
            load_distance: tile_tree.load_distance as f32,
            subdivision_distance: tile_tree.subdivision_distance as f32,
            precision_threshold_distance: tile_tree.precision_threshold_distance as f32,
            morph_range: tile_tree.morph_range,
            blend_range: tile_tree.blend_range,
            view_face: tile_tree.view_face,
            view_lod: tile_tree.view_lod,
            view_coordinates: tile_tree
                .view_coordinates
                .map(|view_coordinate| ViewCoordinate::new(view_coordinate, tile_tree.view_lod)),
            #[cfg(feature = "high_precision")]
            surface_approximation: tile_tree.surface_approximation,
        }
    }
}

pub struct GpuTerrainView {
    terrain_view_buffer: StaticBuffer<TerrainView>,
    approximate_height_buffer: StaticBuffer<f32>,
    approximate_height_readback: Arc<Mutex<f32>>,
    pub(crate) indirect_buffer: StaticBuffer<Indirect>,
    pub(crate) prepare_indirect_bind_group: BindGroup,
    pub(crate) refine_tiles_bind_group: BindGroup,
    pub(crate) terrain_view_bind_group: BindGroup,
}

impl GpuTerrainView {
    fn new(device: &RenderDevice, tile_tree: &TileTree, gpu_tile_tree: &GpuTileTree) -> Self {
        // Todo: figure out a better way of limiting the tile buffer size
        let tile_buffer_size =
            TileCoordinate::min_size().get() * tile_tree.geometry_tile_count as BufferAddress;

        let terrain_view_buffer =
            StaticBuffer::empty(None, device, BufferUsages::STORAGE | BufferUsages::COPY_DST);
        let approximate_height_buffer = StaticBuffer::<f32>::empty(
            None,
            device,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let indirect_buffer =
            StaticBuffer::empty(None, device, BufferUsages::STORAGE | BufferUsages::INDIRECT);
        let parameter_buffer =
            StaticBuffer::<Parameters>::empty(None, device, BufferUsages::STORAGE);
        let temporary_tile_buffer =
            StaticBuffer::<()>::empty_sized(None, device, tile_buffer_size, BufferUsages::STORAGE);
        let final_tile_buffer =
            StaticBuffer::<()>::empty_sized(None, device, tile_buffer_size, BufferUsages::STORAGE);

        let prepare_indirect_bind_group = device.create_bind_group(
            "prepare_indirect_bind_group",
            &create_prepare_indirect_layout(device),
            &BindGroupEntries::single(&indirect_buffer),
        );
        let refine_tiles_bind_group = device.create_bind_group(
            "refine_tiles_bind_group",
            &create_refine_tiles_layout(device),
            &BindGroupEntries::sequential((
                &terrain_view_buffer,
                &approximate_height_buffer,
                &gpu_tile_tree.tile_tree_buffer,
                &final_tile_buffer,
                &temporary_tile_buffer,
                &parameter_buffer,
            )),
        );
        let terrain_view_bind_group = device.create_bind_group(
            "terrain_view_bind_group",
            &create_terrain_view_layout(device),
            &BindGroupEntries::sequential((
                &terrain_view_buffer,
                &approximate_height_buffer,
                &gpu_tile_tree.tile_tree_buffer,
                &final_tile_buffer,
            )),
        );

        Self {
            terrain_view_buffer,
            approximate_height_buffer,
            approximate_height_readback: tile_tree.approximate_height_readback.clone(),
            indirect_buffer,
            prepare_indirect_bind_group,
            refine_tiles_bind_group,
            terrain_view_bind_group,
        }
    }

    pub(crate) fn refinement_count(&self) -> u32 {
        self.terrain_view_buffer.value().refinement_count
    }

    pub(crate) fn initialize(
        device: Res<RenderDevice>,
        mut gpu_terrain_views: ResMut<TerrainViewComponents<GpuTerrainView>>,
        gpu_tile_trees: Res<TerrainViewComponents<GpuTileTree>>,
        tile_trees: Extract<Res<TerrainViewComponents<TileTree>>>,
    ) {
        for (&(atlas_handle, view), tile_tree) in tile_trees.iter() {
            if gpu_terrain_views.contains_key(&(atlas_handle, view)) {
                return;
            }

            let gpu_tile_tree = gpu_tile_trees.get(&(atlas_handle, view)).unwrap();

            gpu_terrain_views.insert(
                (atlas_handle, view),
                GpuTerrainView::new(&device, tile_tree, gpu_tile_tree),
            );
        }
    }

    pub(crate) fn extract(
        mut gpu_terrain_views: ResMut<TerrainViewComponents<GpuTerrainView>>,
        tile_trees: Extract<Res<TerrainViewComponents<TileTree>>>,
    ) {
        for (&(atlas_handle, view), tile_tree) in tile_trees.iter() {
            let gpu_terrain_views = gpu_terrain_views.get_mut(&(atlas_handle, view)).unwrap();

            gpu_terrain_views
                .terrain_view_buffer
                .set_value(TerrainView::from_tile_tree(tile_tree));
        }
    }

    pub(crate) fn prepare(
        queue: Res<RenderQueue>,
        mut gpu_terrain_views: ResMut<TerrainViewComponents<GpuTerrainView>>,
    ) {
        for gpu_terrain_view in &mut gpu_terrain_views.values_mut() {
            gpu_terrain_view.terrain_view_buffer.update(&queue);
        }
    }

    pub(crate) fn readback_view_height(&self, device: &RenderDevice, queue: &RenderQueue) {
        let readback = self.approximate_height_readback.clone();

        DownloadBuffer::read_buffer(
            device.wgpu_device(),
            &queue,
            &self.approximate_height_buffer.slice(..),
            move |result| {
                let buffer = result.expect("Reading buffer failed!");

                *readback.lock().unwrap() = bytemuck::cast_slice::<u8, f32>(&buffer)[0];
            },
        );
    }
}

pub struct SetTerrainViewBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainViewBindGroup<I> {
    type Param = SRes<TerrainViewComponents<GpuTerrainView>>;
    type ViewQuery = Entity;
    type ItemQuery = Read<Handle<TileAtlas>>;

    #[inline]
    fn render<'w>(
        _: &P,
        view: ROQueryItem<'w, Self::ViewQuery>,
        atlas_handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        gpu_terrain_views: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = gpu_terrain_views
            .into_inner()
            .get(&(atlas_handle.unwrap().id(), view))
            .unwrap();

        pass.set_bind_group(I, &data.terrain_view_bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub(crate) struct DrawTerrainCommand;

impl<P: PhaseItem> RenderCommand<P> for DrawTerrainCommand {
    type Param = SRes<TerrainViewComponents<GpuTerrainView>>;
    type ViewQuery = Entity;
    type ItemQuery = Read<Handle<TileAtlas>>;

    #[inline]
    fn render<'w>(
        _: &P,
        view: ROQueryItem<'w, Self::ViewQuery>,
        atlas_handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        gpu_terrain_views: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = gpu_terrain_views
            .into_inner()
            .get(&(atlas_handle.unwrap().id(), view))
            .unwrap();

        pass.draw_indirect(&data.indirect_buffer, 0);

        RenderCommandResult::Success
    }
}
