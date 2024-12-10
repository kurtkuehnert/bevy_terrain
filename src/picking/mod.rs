use crate::{
    big_space::GridCell,
    render::terrain_pass::{TerrainPass, TerrainViewDepthTexture},
    shaders::PICKING_SHADER,
    util::GpuBuffer,
};
use bevy::render::sync_world::RenderEntity;

use bevy::render::{Extract, Render, RenderSet};
use bevy::{
    core_pipeline::core_3d::graph::Core3d,
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{
            self, NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{
                storage_buffer, texture_2d_multisampled, texture_depth_2d_multisampled,
            },
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::ExtractedView,
        RenderApp,
    },
    window::PrimaryWindow,
};
use std::sync::{Arc, Mutex};

pub fn picking_system(
    window: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<(&mut PickingData, &GridCell)>,
) {
    let window = window.single();
    let size = window.size();

    for (mut picking_data, &cell) in &mut camera {
        if let Some(position) = window.cursor_position() {
            picking_data.input = Some(PickingInput {
                cursor_coords: Vec2::new(position.x, size.y - position.y) / window.size(),
                cell,
            });
        } else {
            picking_data.input = None;
        }

        let result = picking_data.readback.lock().unwrap().clone();
        picking_data.result = result;
    }
}

#[derive(Default, Debug, Clone)]
pub struct PickingInput {
    pub cursor_coords: Vec2,
    pub cell: GridCell,
}

#[derive(Default, Debug, Clone)]
pub struct PickingResult {
    pub cursor_coords: Vec2,
    pub cell: GridCell,            // cell of floating origin (camera)
    pub translation: Option<Vec3>, // relative to floating origin cell
    pub world_from_clip: Mat4,
}

#[derive(Component, Default, Clone)]
pub struct PickingData {
    pub input: Option<PickingInput>,
    pub result: PickingResult,
    readback: Arc<Mutex<PickingResult>>,
}

#[derive(Component)]
pub struct GpuPickingData {
    cell: GridCell,
    buffer: GpuBuffer<GpuPickingUniform>,
    readback: Arc<Mutex<PickingResult>>,
    bind_group: Option<BindGroup>,
}

impl GpuPickingData {
    pub(crate) fn new(device: &RenderDevice, picking_data: &PickingData) -> Self {
        let mut picking_buffer = GpuBuffer::empty(
            device,
            BufferUsages::STORAGE
                | BufferUsages::COPY_DST
                | BufferUsages::COPY_SRC
                | BufferUsages::MAP_READ,
        );
        picking_buffer.enable_readback();

        Self {
            cell: GridCell::default(),
            buffer: picking_buffer,
            readback: picking_data.readback.clone(),
            bind_group: None,
        }
    }

    pub(crate) fn initialize(
        mut commands: Commands,
        device: Res<RenderDevice>,
        picking_data: Extract<Query<(RenderEntity, &PickingData), Added<PickingData>>>,
    ) {
        for (render_view, picking_data) in &picking_data {
            commands.insert_or_spawn_batch(
                [(render_view, GpuPickingData::new(&device, picking_data)); 1],
            );
        }
    }

    pub(crate) fn extract(
        picking_data: Extract<Query<(RenderEntity, &PickingData)>>,
        mut gpu_picking_data: Query<&mut GpuPickingData>,
    ) {
        for (render_view, picking_data) in &picking_data {
            let Ok(mut gpu_picking_data) = gpu_picking_data.get_mut(render_view) else {
                continue;
            };

            let Some(input) = picking_data.input.as_ref() else {
                continue;
            };

            gpu_picking_data.cell = input.cell;
            gpu_picking_data.buffer.set_value(GpuPickingUniform {
                cursor_coords: input.cursor_coords,
                depth: 0.0,
                stencil: 255,
            })
        }
    }

    pub(crate) fn prepare(
        device: Res<RenderDevice>,
        queue: Res<RenderQueue>,
        picking_pipeline: Res<PickingPipeline>,
        mut views: Query<(&mut GpuPickingData, &TerrainViewDepthTexture)>,
    ) {
        for (mut gpu_picking_data, depth) in &mut views {
            gpu_picking_data.buffer.update(&queue);

            gpu_picking_data.bind_group = Some(device.create_bind_group(
                None,
                &picking_pipeline.layout,
                &BindGroupEntries::sequential((
                    &gpu_picking_data.buffer,
                    &depth.depth_view,
                    &depth.stencil_view,
                )),
            ));
        }
    }

    pub fn cleanup(mut views: Query<(&mut GpuPickingData, &ExtractedView)>) {
        for (mut gpu_picking_data, extracted_view) in &mut views {
            let world_from_clip = extracted_view.world_from_view.compute_matrix()
                * extracted_view.clip_from_view.inverse();
            let cell = gpu_picking_data.cell;
            let readback = gpu_picking_data.readback.clone();

            gpu_picking_data.buffer.download_readback(move |result| {
                let gpu_picking_data = result.expect("Reading buffer failed!");

                let translation = if gpu_picking_data.depth > 0.0 {
                    let ndc_coords =
                        (gpu_picking_data.cursor_coords * 2.0 - 1.0).extend(gpu_picking_data.depth);
                    Some(world_from_clip.project_point3(ndc_coords))
                } else {
                    None
                };

                // dbg!(gpu_picking_data.cursor_coords);
                // dbg!(0.1 / gpu_picking_data.depth);
                // dbg!(gpu_picking_data.stencil);

                let result = &mut readback.lock().unwrap();
                result.cursor_coords = gpu_picking_data.cursor_coords;
                result.cell = cell;
                result.translation = translation;
                result.world_from_clip = world_from_clip;
            });
        }
    }
}

#[derive(Default, Debug, Clone, ShaderType)]
pub struct GpuPickingUniform {
    pub cursor_coords: Vec2,
    pub depth: f32,
    pub stencil: u32,
}

#[derive(Resource)]
pub struct PickingPipeline {
    id: CachedComputePipelineId,
    layout: BindGroupLayout,
}

impl FromWorld for PickingPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let layout = device.create_bind_group_layout(
            None,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    storage_buffer::<GpuPickingUniform>(false),
                    texture_depth_2d_multisampled(),
                    texture_2d_multisampled(TextureSampleType::Uint),
                ),
            ),
        );

        let id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: world.load_asset(PICKING_SHADER),
            shader_defs: vec![],
            entry_point: "pick".into(),
            zero_initialize_workgroup_memory: false,
        });

        Self { id, layout }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct Picking;

#[derive(Default)]
pub struct PickingNode;

impl render_graph::ViewNode for PickingNode {
    type ViewQuery = &'static GpuPickingData;

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        gpu_picking_data: QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let picking_pipeline = world.resource::<PickingPipeline>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(picking_pipeline.id) else {
            return Ok(());
        };

        let Some(bind_group) = gpu_picking_data.bind_group.as_ref() else {
            return Ok(());
        };

        render_context.add_command_buffer_generation_task(move |device| {
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_pipeline(pipeline);
            pass.dispatch_workgroups(1, 1, 1);
            drop(pass);

            gpu_picking_data
                .buffer
                .copy_to_readback(&device, &mut encoder);

            encoder.finish()
        });

        Ok(())
    }
}

pub struct TerrainPickingPlugin;

impl Plugin for TerrainPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, picking_system);

        app.sub_app_mut(RenderApp)
            .add_systems(
                ExtractSchedule,
                (
                    GpuPickingData::initialize,
                    GpuPickingData::extract.after(GpuPickingData::initialize),
                ),
            )
            .add_systems(
                Render,
                (
                    GpuPickingData::prepare.in_set(RenderSet::Prepare),
                    GpuPickingData::cleanup
                        .before(World::clear_entities)
                        .in_set(RenderSet::Cleanup),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<PickingNode>>(Core3d, Picking)
            .add_render_graph_edge(Core3d, TerrainPass, Picking);
    }
    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<PickingPipeline>();
    }
}
