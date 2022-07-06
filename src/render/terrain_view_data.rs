use crate::{
    render::layouts::{
        INDIRECT_BUFFER_SIZE, PARAMETER_BUFFER_SIZE, PATCH_SIZE, TERRAIN_VIEW_CONFIG_SIZE,
    },
    terrain::Terrain,
    terrain_view::{TerrainView, TerrainViewConfig},
    GpuQuadtree, TerrainComputePipelines, TerrainRenderPipeline, TerrainViewComponents,
};
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    render::{
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        RenderWorld,
    },
};

// Todo: consider factoring out the tesselation
pub struct TerrainViewData {
    pub(crate) indirect_buffer: Buffer,
    pub(crate) view_config_buffer: Buffer,
    pub(crate) prepare_indirect_bind_group: BindGroup,
    pub(crate) tessellation_bind_group: BindGroup,
    pub(crate) terrain_view_bind_group: BindGroup,
}

impl TerrainViewData {
    fn new(
        device: &RenderDevice,
        view_config: &TerrainViewConfig,
        gpu_quadtree: &GpuQuadtree,
        terrain_pipeline: &TerrainRenderPipeline,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Self {
        let indirect_buffer = Self::create_indirect_buffer(device);
        let view_config_buffer = Self::create_view_config_buffer(device);
        let parameter_buffer = Self::create_parameter_buffer(device);
        let (temporary_patch_buffer, final_patch_buffer) =
            Self::create_patch_buffers(device, view_config);

        let prepare_indirect_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "prepare_indirect_bind_group".into(),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: indirect_buffer.as_entire_binding(),
            }],
            layout: &compute_pipelines.prepare_indirect_layout,
        });
        let tessellation_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "tessellation_bind_group".into(),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: parameter_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: temporary_patch_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: final_patch_buffer.as_entire_binding(),
                },
            ],
            layout: &compute_pipelines.tessellation_layout,
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
                    resource: BindingResource::TextureView(&gpu_quadtree.quadtree_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: final_patch_buffer.as_entire_binding(),
                },
            ],
            layout: &terrain_pipeline.terrain_view_layout,
        });

        Self {
            indirect_buffer,
            view_config_buffer,
            prepare_indirect_bind_group,
            tessellation_bind_group,
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

    fn create_patch_buffers(
        device: &RenderDevice,
        view_config: &TerrainViewConfig,
    ) -> (Buffer, Buffer) {
        let buffer_descriptor = BufferDescriptor {
            label: "patch_buffer".into(),
            size: 32 + PATCH_SIZE * view_config.patch_count as BufferAddress, // Todo: figure out a better patch buffer size limit
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        (
            device.create_buffer(&buffer_descriptor),
            device.create_buffer(&buffer_descriptor),
        )
    }

    pub(crate) fn update(&self, queue: &RenderQueue, terrain_view_config: &TerrainViewConfig) {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(&terrain_view_config.shader_data()).unwrap();
        queue.write_buffer(&self.view_config_buffer, 0, &buffer.into_inner());
    }
}

pub(crate) fn initialize_terrain_view_data(
    mut render_world: ResMut<RenderWorld>,
    device: Res<RenderDevice>,
    view_configs: Res<TerrainViewComponents<TerrainViewConfig>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<Entity, Added<Terrain>>,
) {
    let mut terrain_view_data = render_world
        .remove_resource::<TerrainViewComponents<TerrainViewData>>()
        .unwrap();

    let gpu_quadtrees = render_world.resource::<TerrainViewComponents<GpuQuadtree>>();
    let terrain_pipeline = render_world.resource::<TerrainRenderPipeline>();
    let compute_pipelines = render_world.resource::<TerrainComputePipelines>();

    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            let view_config = view_configs.get(&(terrain, view)).unwrap();
            let gpu_quadtree = gpu_quadtrees.get(&(terrain, view)).unwrap();

            terrain_view_data.insert(
                (terrain, view),
                TerrainViewData::new(
                    &device,
                    view_config,
                    gpu_quadtree,
                    terrain_pipeline,
                    compute_pipelines,
                ),
            );
        }
    }

    render_world.insert_resource(terrain_view_data);
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
