use crate::{
    render::{
        INDIRECT_BUFFER_SIZE, PARAMETER_BUFFER_SIZE, PREPARE_INDIRECT_LAYOUT, REFINE_TILES_LAYOUT,
        TERRAIN_VIEW_CONFIG_SIZE, TERRAIN_VIEW_LAYOUT, TILE_SIZE,
    },
    terrain::Terrain,
    terrain_view::{TerrainView, TerrainViewConfig},
    TerrainViewComponents,
};
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        Extract,
    },
};

#[derive(Clone, Default, ShaderType)]
pub(crate) struct TerrainViewConfigUniform {
    height_under_viewer: f32,
    node_count: u32,
    tile_count: u32,
    refinement_count: u32,
    tile_scale: f32,
    grid_size: f32,
    vertices_per_row: u32,
    vertices_per_tile: u32,
    morph_distance: f32,
    blend_distance: f32,
    morph_range: f32,
    blend_range: f32,
}

impl From<&TerrainViewConfig> for TerrainViewConfigUniform {
    fn from(config: &TerrainViewConfig) -> Self {
        TerrainViewConfigUniform {
            height_under_viewer: config.height_under_viewer,
            node_count: config.node_count,
            tile_count: config.tile_count,
            refinement_count: config.refinement_count,
            tile_scale: config.tile_scale,
            grid_size: config.grid_size as f32,
            vertices_per_row: 2 * (config.grid_size + 2),
            vertices_per_tile: 2 * config.grid_size * (config.grid_size + 2),
            morph_distance: config.view_distance
                / 2.0_f32.powf(config.additional_refinement as f32 + 1.0),
            blend_distance: config.view_distance,
            morph_range: config.morph_range,
            blend_range: config.blend_range,
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

        let prepare_indirect_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "prepare_indirect_bind_group".into(),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: indirect_buffer.as_entire_binding(),
            }],
            layout: &device.create_bind_group_layout(&PREPARE_INDIRECT_LAYOUT),
        });
        let refine_tiles_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "refine_tiles_bind_group".into(),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&quadtree.texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: final_tile_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: temporary_tile_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: parameter_buffer.as_entire_binding(),
                },
            ],
            layout: &device.create_bind_group_layout(&REFINE_TILES_LAYOUT),
        });
        let terrain_view_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "terrain_view_bind_group".into(),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&quadtree.texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: final_tile_buffer.as_entire_binding(),
                },
            ],
            layout: &device.create_bind_group_layout(&TERRAIN_VIEW_LAYOUT),
        });

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

    pub(crate) fn update(&self, queue: &RenderQueue, view_config: &TerrainViewConfig) {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer
            .write(&TerrainViewConfigUniform::from(view_config))
            .unwrap();
        queue.write_buffer(&self.view_config_buffer, 0, &buffer.into_inner());
    }
}

pub(crate) fn initialize_terrain_view_data(
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

pub struct SetTerrainViewBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetTerrainViewBindGroup<I> {
    type Param = SRes<TerrainViewComponents<TerrainViewData>>;

    #[inline]
    fn render<'w>(
        view: Entity,
        terrain: Entity,
        terrain_view_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_view_data
            .into_inner()
            .get(&(terrain, view))
            .unwrap();

        pass.set_bind_group(I, &data.terrain_view_bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub(crate) struct DrawTerrainCommand;

impl EntityRenderCommand for DrawTerrainCommand {
    type Param = SRes<TerrainViewComponents<TerrainViewData>>;

    #[inline]
    fn render<'w>(
        view: Entity,
        terrain: Entity,
        terrain_view_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_view_data
            .into_inner()
            .get(&(terrain, view))
            .unwrap();

        pass.draw_indirect(&data.indirect_buffer, 0);
        RenderCommandResult::Success
    }
}
