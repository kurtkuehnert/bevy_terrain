use crate::{
    render::{
        INDIRECT_BUFFER_SIZE, PARAMETER_BUFFER_SIZE, PREPARE_INDIRECT_LAYOUT, REFINE_TILES_LAYOUT,
        TERRAIN_VIEW_CONFIG_SIZE, TERRAIN_VIEW_LAYOUT, TILE_SIZE,
    },
    terrain::{Terrain, TerrainConfig},
    terrain_view::{TerrainView, TerrainViewConfig},
    TerrainViewComponents,
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

#[derive(Clone, Default, ShaderType)]
pub(crate) struct TerrainViewConfigUniform {
    height_under_viewer: f32,
    node_count: u32,
    tile_count: u32,
    pub(crate) refinement_count: u32,
    tile_scale: f32,
    grid_size: f32,
    vertices_per_row: u32,
    vertices_per_tile: u32,
    morph_distance: f32,
    blend_distance: f32,
    morph_range: f32,
    blend_range: f32,
}

impl TerrainViewConfigUniform {
    fn new(config: &TerrainConfig, view_config: &TerrainViewConfig) -> Self {
        let view_distance = view_config.view_distance * config.leaf_node_size as f32;

        TerrainViewConfigUniform {
            height_under_viewer: view_config.height_under_viewer,
            node_count: view_config.node_count,
            tile_count: view_config.tile_count,
            refinement_count: view_config.refinement_count,
            tile_scale: view_config.tile_scale,
            grid_size: view_config.grid_size as f32,
            vertices_per_row: 2 * (view_config.grid_size + 2),
            vertices_per_tile: 2 * view_config.grid_size * (view_config.grid_size + 2),
            morph_distance: view_distance
                / 2.0_f32.powf(view_config.additional_refinement as f32 + 1.0),
            blend_distance: view_distance,
            morph_range: view_config.morph_range,
            blend_range: view_config.blend_range,
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

    pub(crate) fn update(
        &self,
        queue: &RenderQueue,
        view_config_uniform: &TerrainViewConfigUniform,
    ) {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(view_config_uniform).unwrap();
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

pub(crate) fn extract_terrain_view_config(
    mut view_config_uniforms: ResMut<TerrainViewComponents<TerrainViewConfigUniform>>,
    configs: Extract<Query<&TerrainConfig>>,
    view_configs: Extract<Res<TerrainViewComponents<TerrainViewConfig>>>,
) {
    for (&(terrain, view), view_config) in &view_configs.0 {
        let config = configs.get(terrain).unwrap();
        view_config_uniforms.insert(
            (terrain, view),
            TerrainViewConfigUniform::new(config, view_config),
        )
    }
}

pub(crate) fn prepare_terrain_view_config(
    queue: Res<RenderQueue>,
    mut terrain_view_data: ResMut<TerrainViewComponents<TerrainViewData>>,
    view_config_uniforms: Res<TerrainViewComponents<TerrainViewConfigUniform>>,
) {
    for (&(terrain, view), data) in &mut terrain_view_data.0 {
        let view_config_uniform = view_config_uniforms.get(&(terrain, view)).unwrap();
        data.update(&queue, view_config_uniform)
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

        //consider removing this unwrap! 


         match terrain_view_data
            .into_inner()
            .get(&(item.entity(), view)) {

                Some(terrain_data) => {
                   
                    pass.set_bind_group(I, &terrain_data.terrain_view_bind_group, &[]);
                    RenderCommandResult::Success

                }, 
                None => {
                    panic!("Missing terrain data ");
                    println!("WARN: Could not render terrain data");
                    //pass.set_bind_group(I, &data.terrain_view_bind_group, &[]);
                    RenderCommandResult::Failure
                }

            } 
             

      
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
