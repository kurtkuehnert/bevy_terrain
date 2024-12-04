use crate::prelude::TerrainViewComponents;
use crate::{shaders::PICKING_SHADER, util::StaticBuffer};
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
            binding_types::{sampler, storage_buffer, texture_depth_2d_multisampled},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::{ExtractedView, ViewDepthTexture},
        RenderApp,
    },
    window::PrimaryWindow,
};
use ndarray::AssignElem;
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};
use wgpu::util::DownloadBuffer;

pub fn picking_system(
    mut picking_readbacks: ResMut<TerrainViewComponents<PickingReadback>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    for readback in picking_readbacks.values_mut() {
        let data = if let Ok(mut new_data) = readback.readback.lock() {
            let readback_data = new_data.clone();

            let window = window.single();

            if let Some(position) = window.cursor_position() {
                new_data.cursor_coords = position / window.size();
            }

            readback_data
        } else {
            return;
        };

        readback.data.assign_elem(data);
    }
}

#[derive(Default, Clone)]
pub struct PickingReadback {
    pub data: PickingData,
    readback: Arc<Mutex<PickingData>>,
}

fn extract_picking_readback(
    picking_readbacks: Extract<Res<TerrainViewComponents<PickingReadback>>>,
    mut gpu_picking_readbacks: ResMut<TerrainViewComponents<PickingReadback>>,
) {
    **gpu_picking_readbacks = picking_readbacks.clone();
}

#[derive(Default, Debug, Clone, ShaderType)]
pub struct PickingData {
    pub cursor_coords: Vec2,
    pub depth: f32,
    pub world_position: Vec3,
    pub world_from_clip: Mat4,
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
                    storage_buffer::<PickingData>(false),
                    sampler(SamplerBindingType::Filtering),
                    // texture_depth_2d(),
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
        let picking_readbacks = world.resource::<TerrainViewComponents<PickingReadback>>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(picking_pipeline.id) else {
            return Ok(());
        };

        for (&(_terrain, picking_view), picking_readback) in picking_readbacks.iter() {
            if view != picking_view {
                continue;
            }

            let picking_data = picking_readback.readback.lock().unwrap().clone();

            let cursor_coords = Vec2::new(
                picking_data.cursor_coords.x,
                1.0 - picking_data.cursor_coords.y,
            );
            let world_from_clip = extracted_view.world_from_view.compute_matrix()
                * extracted_view.clip_from_view.inverse();

            let picking_buffer = StaticBuffer::<PickingData>::create(
                None,
                device,
                &picking_data,
                BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            );

            let sampler = device.create_sampler(&SamplerDescriptor {
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..default()
            });
            let depth_view = depth.texture.create_view(&Default::default());

            let bind_group = device.create_bind_group(
                None,
                &picking_pipeline.picking_layout,
                &BindGroupEntries::sequential((&picking_buffer, &sampler, &depth_view)),
            );

            let mut command_encoder =
                device.create_command_encoder(&CommandEncoderDescriptor::default());

            let mut compute_pass =
                command_encoder.begin_compute_pass(&ComputePassDescriptor::default());

            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.set_pipeline(pipeline);
            compute_pass.dispatch_workgroups(1, 1, 1);

            drop(compute_pass);

            let command_buffer = command_encoder.finish();

            queue.submit(Some(command_buffer));

            let readback = picking_readback.readback.clone();

            DownloadBuffer::read_buffer(
                device.wgpu_device(),
                queue,
                &picking_buffer.slice(..),
                move |result| {
                    let buffer = result.expect("Reading buffer failed!");
                    let storage_buffer = encase::StorageBuffer::new(buffer.deref());

                    let mut data = PickingData::default();
                    storage_buffer.read(&mut data).unwrap();

                    let ndc_coords = (cursor_coords * 2.0 - 1.0).extend(data.depth);

                    data.world_position = world_from_clip.project_point3(ndc_coords);
                    data.world_from_clip = world_from_clip;

                    *readback.lock().unwrap() = data;
                },
            );
        }

        Ok(())
    }
}

pub struct TerrainPickingPlugin;

impl Plugin for TerrainPickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainViewComponents<PickingReadback>>()
            .add_systems(Update, picking_system);

        app.sub_app_mut(RenderApp)
            .init_resource::<TerrainViewComponents<PickingReadback>>()
            .add_systems(ExtractSchedule, extract_picking_readback)
            .add_render_graph_node::<ViewNodeRunner<PickingNode>>(Core3d, PickingLabel)
            .add_render_graph_edge(Core3d, Node3d::EndMainPass, PickingLabel);
    }
    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<PickingPipeline>();
    }
}
