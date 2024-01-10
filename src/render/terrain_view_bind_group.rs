use crate::{
    terrain::Terrain,
    terrain_data::gpu_quadtree::GpuQuadtree,
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
    util::StaticBuffer,
};
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

const TILE_SIZE: BufferAddress = 16 * 4;
const TILE_BUFFER_MIN_SIZE: Option<BufferSize> = BufferSize::new(32 + TILE_SIZE);

pub(crate) fn create_prepare_indirect_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::single(
            ShaderStages::COMPUTE,
            storage_buffer::<Indirect>(false), // indirect buffer
        ),
    )
}

pub(crate) fn create_refine_tiles_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<TerrainViewConfigUniform>(false), // terrain view config
                storage_buffer_sized(false, None),                 // quadtree
                storage_buffer_sized(false, TILE_BUFFER_MIN_SIZE), // final tiles
                storage_buffer_sized(false, TILE_BUFFER_MIN_SIZE), // temporary tiles
                storage_buffer::<Parameters>(false),               // parameters
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
                uniform_buffer::<TerrainViewConfigUniform>(false), // terrain view config
                storage_buffer_sized(false, None),                 // quadtree
                storage_buffer_read_only_sized(false, TILE_BUFFER_MIN_SIZE), // tiles
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
struct TerrainViewConfigUniform {
    view_local_position: Vec3,
    approximate_height: f32,
    quadtree_size: u32,
    tile_count: u32,
    refinement_count: u32,
    grid_size: f32,
    vertices_per_row: u32,
    vertices_per_tile: u32,
    morph_distance: f32,
    blend_distance: f32,
    morph_range: f32,
    blend_range: f32,
    _padding: Vec2,
}

impl TerrainViewConfigUniform {
    fn new(view_config: &TerrainViewConfig, view_local_position: Vec3) -> Self {
        TerrainViewConfigUniform {
            view_local_position,
            approximate_height: view_config.approximate_height,
            quadtree_size: view_config.quadtree_size,
            tile_count: view_config.tile_count,
            refinement_count: view_config.refinement_count,
            grid_size: view_config.grid_size as f32,
            vertices_per_row: 2 * (view_config.grid_size + 2),
            vertices_per_tile: 2 * view_config.grid_size * (view_config.grid_size + 2),
            morph_distance: view_config.morph_distance,
            blend_distance: view_config.blend_distance,
            morph_range: view_config.morph_range,
            blend_range: view_config.blend_range,
            _padding: Vec2::ZERO,
        }
    }
}

pub struct TerrainViewData {
    view_config_uniform: TerrainViewConfigUniform,
    view_config_buffer: StaticBuffer<TerrainViewConfigUniform>,
    pub(super) indirect_buffer: StaticBuffer<Indirect>,
    pub(super) prepare_indirect_bind_group: BindGroup,
    pub(super) refine_tiles_bind_group: BindGroup,
    pub(super) terrain_view_bind_group: BindGroup,
}

impl TerrainViewData {
    fn new(
        device: &RenderDevice,
        view_config: &TerrainViewConfig,
        gpu_quadtree: &GpuQuadtree,
    ) -> Self {
        // Todo: figure out a better way of limiting the tile buffer size
        let tile_buffer_size = 32 + TILE_SIZE * view_config.tile_count as BufferAddress;

        let view_config_buffer =
            StaticBuffer::empty(device, BufferUsages::UNIFORM | BufferUsages::COPY_DST);
        let indirect_buffer =
            StaticBuffer::empty(device, BufferUsages::STORAGE | BufferUsages::INDIRECT);
        let parameter_buffer = StaticBuffer::<Parameters>::empty(device, BufferUsages::STORAGE);
        let temporary_tile_buffer =
            StaticBuffer::<()>::empty_sized(device, tile_buffer_size, BufferUsages::STORAGE);
        let final_tile_buffer =
            StaticBuffer::<()>::empty_sized(device, tile_buffer_size, BufferUsages::STORAGE);

        let prepare_indirect_bind_group = device.create_bind_group(
            "prepare_indirect_bind_group",
            &create_prepare_indirect_layout(device),
            &BindGroupEntries::single(&indirect_buffer),
        );
        let refine_tiles_bind_group = device.create_bind_group(
            "refine_tiles_bind_group",
            &create_refine_tiles_layout(device),
            &BindGroupEntries::sequential((
                &view_config_buffer,
                &gpu_quadtree.quadtree_buffer,
                &final_tile_buffer,
                &temporary_tile_buffer,
                &parameter_buffer,
            )),
        );
        let terrain_view_bind_group = device.create_bind_group(
            "terrain_view_bind_group",
            &create_terrain_view_layout(device),
            &BindGroupEntries::sequential((
                &view_config_buffer,
                &gpu_quadtree.quadtree_buffer,
                &final_tile_buffer,
            )),
        );

        Self {
            view_config_uniform: default(),
            view_config_buffer,
            indirect_buffer,
            prepare_indirect_bind_group,
            refine_tiles_bind_group,
            terrain_view_bind_group,
        }
    }

    pub(super) fn refinement_count(&self) -> u32 {
        self.view_config_uniform.refinement_count
    }

    pub(crate) fn initialize(
        device: Res<RenderDevice>,
        mut terrain_view_data: ResMut<TerrainViewComponents<TerrainViewData>>,
        gpu_quadtrees: Res<TerrainViewComponents<GpuQuadtree>>,
        view_configs: Extract<Res<TerrainViewComponents<TerrainViewConfig>>>,
        view_query: Extract<Query<Entity, With<TerrainView>>>,
        terrain_query: Extract<Query<Entity, Added<Terrain>>>,
    ) {
        for terrain in &terrain_query {
            for view in &view_query {
                let view_config = view_configs.get(&(terrain, view)).unwrap();
                let gpu_quadtree = gpu_quadtrees.get(&(terrain, view)).unwrap();

                terrain_view_data.insert(
                    (terrain, view),
                    TerrainViewData::new(&device, view_config, gpu_quadtree),
                );
            }
        }
    }

    pub(crate) fn extract(
        mut terrain_view_data: ResMut<TerrainViewComponents<TerrainViewData>>,
        view_configs: Extract<Res<TerrainViewComponents<TerrainViewConfig>>>,
        view_query: Extract<Query<&GlobalTransform, With<TerrainView>>>,
        terrain_query: Extract<Query<&GlobalTransform, With<Terrain>>>,
    ) {
        for (&(terrain, view), view_config) in &view_configs.0 {
            let view_world_position = view_query.get(view).unwrap();
            let terrain_transform = terrain_query.get(terrain).unwrap();
            let terrain_view_data = terrain_view_data.get_mut(&(terrain, view)).unwrap();

            let view_local_position = (terrain_transform.compute_matrix().inverse()
                * view_world_position.translation().extend(1.0))
            .xyz();

            terrain_view_data.view_config_uniform =
                TerrainViewConfigUniform::new(view_config, view_local_position);
        }
    }

    pub(crate) fn prepare(
        queue: Res<RenderQueue>,
        mut terrain_view_data: ResMut<TerrainViewComponents<TerrainViewData>>,
    ) {
        for data in &mut terrain_view_data.0.values_mut() {
            data.view_config_buffer
                .update(&queue, &data.view_config_uniform);
        }
    }
}

pub struct SetTerrainViewBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainViewBindGroup<I> {
    type Param = SRes<TerrainViewComponents<TerrainViewData>>;
    type ViewWorldQuery = Entity;
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        terrain_view_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_view_data
            .into_inner()
            .get(&(item.entity(), view))
            .unwrap();

        pass.set_bind_group(I, &data.terrain_view_bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub(crate) struct DrawTerrainCommand;

impl<P: PhaseItem> RenderCommand<P> for DrawTerrainCommand {
    type Param = SRes<TerrainViewComponents<TerrainViewData>>;
    type ViewWorldQuery = Entity;
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        terrain_view_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_view_data
            .into_inner()
            .get(&(item.entity(), view))
            .unwrap();

        pass.draw_indirect(&data.indirect_buffer, 0);
        RenderCommandResult::Success
    }
}
