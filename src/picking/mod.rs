use crate::big_space::{GridCell, ReferenceFrames};
use crate::prelude::TerrainViewComponents;
use crate::{shaders::PICKING_SHADER, util::StaticBuffer};
use bevy::color::palettes::basic;
use bevy::render::sync_world::MainEntity;
use bevy::render::Extract;
use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{
            self, NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{storage_buffer, texture_depth_2d_multisampled},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::{ExtractedView, ViewDepthTexture},
        RenderApp,
    },
    window::PrimaryWindow,
};
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};
use wgpu::util::DownloadBuffer;

pub fn picking_system(
    frames: ReferenceFrames,
    mut picking_data: ResMut<TerrainViewComponents<PickingData>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = window.single();
    let size = window.size();

    for (&(terrain, _view), picking_data) in picking_data.iter_mut() {
        let frame = frames.parent_frame(terrain).unwrap();
        let origin = frame.local_floating_origin();

        if let Some(position) = window.cursor_position() {
            picking_data.input = Some(PickingInput {
                cursor_coords: Vec2::new(position.x, size.y - position.y) / window.size(),
                cell: origin.cell(),
            });
        } else {
            picking_data.input = None;
        }

        picking_data.result = picking_data.readback.lock().unwrap().clone();
    }
}

#[derive(Default, Clone)]
pub struct PickingData {
    pub input: Option<PickingInput>,
    pub result: PickingResult,
    readback: Arc<Mutex<PickingResult>>,
}

fn extract_picking_data(
    picking_data: Extract<Res<TerrainViewComponents<PickingData>>>,
    mut gpu_picking_data: ResMut<TerrainViewComponents<PickingData>>,
) {
    **gpu_picking_data = picking_data.clone();
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

#[derive(Default, Debug, Clone, ShaderType)]
pub struct GpuPickingData {
    pub cursor_coords: Vec2,
    pub depth: f32,
}

#[derive(Resource)]
pub struct PickingPipeline {
    picking_layout: BindGroupLayout,
    id: CachedComputePipelineId,
}

impl FromWorld for PickingPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let picking_layout = device.create_bind_group_layout(
            None,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    storage_buffer::<GpuPickingData>(false),
                    texture_depth_2d_multisampled(),
                ),
            ),
        );

        let id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![picking_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: world.load_asset(PICKING_SHADER),
            shader_defs: vec![],
            entry_point: "pick".into(),
            zero_initialize_workgroup_memory: false,
        });

        Self { picking_layout, id }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PickingLabel;

#[derive(Default)]
pub struct PickingNode;

impl render_graph::ViewNode for PickingNode {
    type ViewQuery = (
        MainEntity,
        &'static ExtractedView,
        &'static ViewDepthTexture,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        (view, extracted_view, depth): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let picking_pipeline = world.resource::<PickingPipeline>();
        let picking_data = world.resource::<TerrainViewComponents<PickingData>>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(picking_pipeline.id) else {
            return Ok(());
        };

        for (&(_terrain, picking_view), picking_data) in picking_data.iter() {
            if view != picking_view {
                continue;
            }

            let Some(picking_input) = &picking_data.input else {
                continue;
            };

            let world_from_clip = extracted_view.world_from_view.compute_matrix()
                * extracted_view.clip_from_view.inverse();
            let mut gpu_picking_data = GpuPickingData {
                cursor_coords: picking_input.cursor_coords,
                depth: 0.0,
            };
            let cell = picking_input.cell;
            let readback = picking_data.readback.clone();

            let picking_buffer = StaticBuffer::<GpuPickingData>::create(
                None,
                device,
                &gpu_picking_data,
                BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            );

            let depth_view = depth.texture.create_view(&Default::default());

            let bind_group = device.create_bind_group(
                None,
                &picking_pipeline.picking_layout,
                &BindGroupEntries::sequential((&picking_buffer, &depth_view)),
            );

            let mut command_encoder =
                device.create_command_encoder(&CommandEncoderDescriptor::default());

            {
                let mut compute_pass =
                    command_encoder.begin_compute_pass(&ComputePassDescriptor::default());
                compute_pass.set_bind_group(0, &bind_group, &[]);
                compute_pass.set_pipeline(pipeline);
                compute_pass.dispatch_workgroups(1, 1, 1);
            }

            queue.submit(Some(command_encoder.finish()));

            DownloadBuffer::read_buffer(
                device.wgpu_device(),
                queue,
                &picking_buffer.slice(..),
                move |result| {
                    let buffer = result.expect("Reading buffer failed!");
                    let storage_buffer = encase::StorageBuffer::new(buffer.deref());
                    storage_buffer.read(&mut gpu_picking_data).unwrap();

                    let translation = if gpu_picking_data.depth > 0.0 {
                        let ndc_coords = (gpu_picking_data.cursor_coords * 2.0 - 1.0)
                            .extend(gpu_picking_data.depth);
                        Some(world_from_clip.project_point3(ndc_coords))
                    } else {
                        None
                    };

                    let result = &mut readback.lock().unwrap();
                    result.cursor_coords = gpu_picking_data.cursor_coords;
                    result.cell = cell;
                    result.translation = translation;
                    result.world_from_clip = world_from_clip;
                },
            );
        }

        Ok(())
    }
}

pub struct TerrainPickingPlugin;

impl Plugin for TerrainPickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainViewComponents<PickingData>>()
            .add_systems(Update, picking_system);

        app.sub_app_mut(RenderApp)
            .init_resource::<TerrainViewComponents<PickingData>>()
            .add_systems(ExtractSchedule, extract_picking_data)
            .add_render_graph_node::<ViewNodeRunner<PickingNode>>(Core3d, PickingLabel)
            .add_render_graph_edge(Core3d, Node3d::EndMainPass, PickingLabel);
    }
    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<PickingPipeline>();
    }
}
