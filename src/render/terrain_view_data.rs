use crate::{
    render::{
        INDIRECT_BUFFER_SIZE, PARAMETER_BUFFER_SIZE, PREPARE_INDIRECT_LAYOUT, REFINE_TILES_LAYOUT,
        TERRAIN_VIEW_CONFIG_SIZE, TERRAIN_VIEW_LAYOUT, TILE_SIZE,
    },
    terrain::Terrain,
    terrain_view::{TerrainView, TerrainViewComponents, TerrainViewConfig},
};
use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        Extract,
    },
};

// Todo: clean up this file similar to terrain_bind_groups.rs, once buffers can be shared in the
// AsBindGroup derive macro

#[derive(Clone, Default, ShaderType)]
pub(crate) struct TerrainViewConfigUniform {
    view_local_position: Vec3,
    height_under_viewer: f32,
    quadtree_size: u32,
    tile_count: u32,
    pub(crate) refinement_count: u32,
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
            height_under_viewer: view_config.height_under_viewer,
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
    pub(crate) indirect_buffer: Buffer,
    pub(crate) view_config_buffer: Buffer,
    pub(crate) prepare_indirect_bind_group: BindGroup,
    pub(crate) refine_tiles_bind_group: BindGroup,
    pub(crate) terrain_view_bind_group: BindGroup,
}

impl TerrainViewData {
    fn new(
        device: &RenderDevice,
        images: &RenderAssets<Image>,
        view_config: &TerrainViewConfig,
    ) -> Self {
        let indirect_buffer = Self::create_indirect_buffer(device);
        let view_config_buffer = Self::create_view_config_buffer(device);
        let parameter_buffer = Self::create_parameter_buffer(device);
        let (temporary_tile_buffer, final_tile_buffer) =
            Self::create_tile_buffers(device, view_config);

        let quadtree = images.get(&view_config.quadtree_handle).unwrap();

        let prepare_indirect_bind_group = device.create_bind_group(
            "prepare_indirect_bind_group",
            &device.create_bind_group_layout(&PREPARE_INDIRECT_LAYOUT),
            &BindGroupEntries::single(indirect_buffer.as_entire_binding()),
        );
        let refine_tiles_bind_group = device.create_bind_group(
            "refine_tiles_bind_group",
            &device.create_bind_group_layout(&REFINE_TILES_LAYOUT),
            &BindGroupEntries::sequential((
                view_config_buffer.as_entire_binding(),
                &quadtree.texture_view,
                final_tile_buffer.as_entire_binding(),
                temporary_tile_buffer.as_entire_binding(),
                parameter_buffer.as_entire_binding(),
            )),
        );

        let terrain_view_bind_group = device.create_bind_group(
            "terrain_view_bind_group",
            &device.create_bind_group_layout(&TERRAIN_VIEW_LAYOUT),
            &BindGroupEntries::sequential((
                view_config_buffer.as_entire_binding(),
                &quadtree.texture_view,
                final_tile_buffer.as_entire_binding(),
            )),
        );

        Self {
            indirect_buffer,
            view_config_buffer,
            prepare_indirect_bind_group,
            refine_tiles_bind_group,
            terrain_view_bind_group,
        }
    }

    fn create_view_config_buffer(device: &RenderDevice) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: "view_config_buffer".into(),
            size: TERRAIN_VIEW_CONFIG_SIZE,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_indirect_buffer(device: &RenderDevice) -> Buffer {
        device.create_buffer_with_data(&BufferInitDescriptor {
            label: "indirect_buffer".into(),
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            contents: &[0; INDIRECT_BUFFER_SIZE as usize],
        })
    }
    fn create_parameter_buffer(device: &RenderDevice) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: "parameter_buffer".into(),
            size: PARAMETER_BUFFER_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    fn create_tile_buffers(
        device: &RenderDevice,
        view_config: &TerrainViewConfig,
    ) -> (Buffer, Buffer) {
        let buffer_descriptor = BufferDescriptor {
            label: "tile_buffer".into(),
            size: TILE_SIZE * view_config.tile_count as BufferAddress, // Todo: figure out a better tile buffer size limit
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        (
            device.create_buffer(&buffer_descriptor),
            device.create_buffer(&buffer_descriptor),
        )
    }

    pub(crate) fn update(
        &self,
        queue: &RenderQueue,
        view_config_uniform: &TerrainViewConfigUniform,
    ) {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(view_config_uniform).unwrap();
        queue.write_buffer(&self.view_config_buffer, 0, &buffer.into_inner());
    }

    pub(crate) fn initialize(
        device: Res<RenderDevice>,
        images: Res<RenderAssets<Image>>,
        mut terrain_view_data: ResMut<TerrainViewComponents<TerrainViewData>>,
        view_configs: Extract<Res<TerrainViewComponents<TerrainViewConfig>>>,
        view_query: Extract<Query<Entity, With<TerrainView>>>,
        terrain_query: Extract<Query<Entity, Added<Terrain>>>,
    ) {
        for terrain in terrain_query.iter() {
            for view in view_query.iter() {
                let view_config = view_configs.get(&(terrain, view)).unwrap();

                terrain_view_data.insert(
                    (terrain, view),
                    TerrainViewData::new(&device, &images, view_config),
                );
            }
        }
    }

    pub(crate) fn extract(
        mut view_config_uniforms: ResMut<TerrainViewComponents<TerrainViewConfigUniform>>,
        view_configs: Extract<Res<TerrainViewComponents<TerrainViewConfig>>>,
        view_query: Extract<Query<&GlobalTransform, With<TerrainView>>>,
        terrain_query: Extract<Query<&GlobalTransform, With<Terrain>>>,
    ) {
        for (&(terrain, view), view_config) in &view_configs.0 {
            let view_world_position = view_query.get(view).unwrap().translation();
            let terrain_transform = terrain_query.get(terrain).unwrap();
            let model = terrain_transform.compute_matrix();
            let inverse_model = model.inverse();

            let view_local_position = (inverse_model * view_world_position.extend(1.0)).xyz();
            view_config_uniforms.insert(
                (terrain, view),
                TerrainViewConfigUniform::new(view_config, view_local_position),
            )
        }
    }

    pub(crate) fn prepare(
        queue: Res<RenderQueue>,
        mut terrain_view_data: ResMut<TerrainViewComponents<TerrainViewData>>,
        view_config_uniforms: Res<TerrainViewComponents<TerrainViewConfigUniform>>,
    ) {
        for (&(terrain, view), data) in &mut terrain_view_data.0 {
            let view_config_uniform = view_config_uniforms.get(&(terrain, view)).unwrap();
            data.update(&queue, view_config_uniform)
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
