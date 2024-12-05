use crate::{
    big_space::GridCell, render::terrain_pass::TerrainViewDepthTexture, shaders::PICKING_SHADER,
    util::StaticBuffer,
};
use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
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
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};
use wgpu::util::DownloadBuffer;

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

#[derive(Component, ExtractComponent, Default, Clone)]
pub struct PickingData {
    pub input: Option<PickingInput>,
    pub result: PickingResult,
    readback: Arc<Mutex<PickingResult>>,
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
    pub stencil: u32,
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
                    texture_2d_multisampled(TextureSampleType::Uint),
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
pub struct Picking;

#[derive(Default)]
pub struct PickingNode;

impl render_graph::ViewNode for PickingNode {
    type ViewQuery = (
        &'static PickingData,
        &'static ExtractedView,
        &'static TerrainViewDepthTexture,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        (picking_data, extracted_view, depth): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let picking_pipeline = world.resource::<PickingPipeline>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(picking_pipeline.id) else {
            return Ok(());
        };

        let Some(picking_input) = &picking_data.input else {
            return Ok(());
        };

        let world_from_clip = extracted_view.world_from_view.compute_matrix()
            * extracted_view.clip_from_view.inverse();
        let mut gpu_picking_data = GpuPickingData {
            cursor_coords: picking_input.cursor_coords,
            depth: 0.0,
            stencil: u8::MAX as u32,
        };
        let cell = picking_input.cell;
        let readback = picking_data.readback.clone();

        let picking_buffer = StaticBuffer::<GpuPickingData>::create(
            None,
            device,
            &gpu_picking_data,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        );

        let depth_view = depth.texture.create_view(&TextureViewDescriptor {
            aspect: TextureAspect::DepthOnly,
            ..default()
        });

        let stencil_view = depth.texture.create_view(&TextureViewDescriptor {
            aspect: TextureAspect::StencilOnly,
            ..default()
        });

        let bind_group = device.create_bind_group(
            None,
            &picking_pipeline.picking_layout,
            &BindGroupEntries::sequential((&picking_buffer, &depth_view, &stencil_view)),
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
                    let ndc_coords =
                        (gpu_picking_data.cursor_coords * 2.0 - 1.0).extend(gpu_picking_data.depth);
                    Some(world_from_clip.project_point3(ndc_coords))
                } else {
                    None
                };

                // dbg!(0.1 / gpu_picking_data.depth);
                // dbg!(gpu_picking_data.stencil);

                let result = &mut readback.lock().unwrap();
                result.cursor_coords = gpu_picking_data.cursor_coords;
                result.cell = cell;
                result.translation = translation;
                result.world_from_clip = world_from_clip;
            },
        );

        Ok(())
    }
}

pub struct TerrainPickingPlugin;

impl Plugin for TerrainPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<PickingData>::default())
            .add_systems(Update, picking_system);

        app.sub_app_mut(RenderApp)
            .add_render_graph_node::<ViewNodeRunner<PickingNode>>(Core3d, Picking)
            .add_render_graph_edge(Core3d, Node3d::EndMainPass, Picking);
    }
    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<PickingPipeline>();
    }
}
