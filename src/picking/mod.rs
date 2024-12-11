use crate::{
    big_space::GridCell,
    render::terrain_pass::{TerrainPassNode, TerrainViewDepthTexture},
    shaders::PICKING_SHADER,
};
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::core_3d::graph::Core3d,
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        gpu_readback::{Readback, ReadbackComplete},
        render_asset::RenderAssets,
        render_graph::{
            self, NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{
                storage_buffer, texture_2d_multisampled, texture_depth_2d_multisampled,
            },
            *,
        },
        renderer::{RenderContext, RenderDevice},
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
        RenderApp,
    },
    window::PrimaryWindow,
};

pub fn picking_system(
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform, &GridCell, &PickingData)>,
) {
    let Ok(window) = window.get_single() else {
        return;
    };
    let Some(position) = window.cursor_position() else {
        return;
    };
    let cursor_coords = Vec2::new(position.x, window.size().y - position.y) / window.size();

    for (camera, global_transform, &cell, picking_data) in &camera {
        let buffer = buffers.get_mut(&picking_data.buffer).unwrap();
        buffer.set_data(GpuPickingData {
            cursor_coords,
            depth: 0.0,
            stencil: 255,
            world_from_clip: global_transform.compute_matrix() * camera.clip_from_view().inverse(),
            cell: IVec3::new(cell.x, cell.y, cell.z),
        });
    }
}

pub fn picking_readback(
    trigger: Trigger<ReadbackComplete>,
    mut picking_data: Query<&mut PickingData>,
) {
    let GpuPickingData {
        cursor_coords,
        depth,
        stencil: _stencil,
        world_from_clip,
        cell,
    } = trigger.event().to_shader_type();

    let ndc_coords = (2.0 * cursor_coords - 1.0).extend(depth);

    let mut picking_data = picking_data.get_mut(trigger.entity()).unwrap();
    picking_data.cursor_coords = cursor_coords;
    picking_data.cell = GridCell::new(cell.x, cell.y, cell.z);
    picking_data.translation = (depth > 0.0).then(|| world_from_clip.project_point3(ndc_coords));
    picking_data.world_from_clip = world_from_clip;

    // dbg!(cursor_coords);
    // dbg!(1.0 / depth);
    // dbg!(stencil);
}

pub fn picking_hook(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let mut buffers = world.resource_mut::<Assets<ShaderStorageBuffer>>();
    let mut buffer = ShaderStorageBuffer::with_size(
        GpuPickingData::min_size().get() as usize,
        RenderAssetUsages::default(),
    );
    buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
    let buffer = buffers.add(buffer);

    world
        .commands()
        .entity(entity)
        .insert(Readback::buffer(buffer.clone_weak()))
        .observe(picking_readback);

    let mut picking_data = world.get_mut::<PickingData>(entity).unwrap();
    picking_data.buffer = buffer;
}

#[derive(Default, Clone, Component)]
#[component(on_add = picking_hook)]
pub struct PickingData {
    pub cursor_coords: Vec2,
    pub cell: GridCell,            // cell of floating origin (camera)
    pub translation: Option<Vec3>, // relative to floating origin cell
    pub world_from_clip: Mat4,
    buffer: Handle<ShaderStorageBuffer>,
}

impl ExtractComponent for PickingData {
    type QueryData = &'static PickingData;
    type QueryFilter = ();
    type Out = GpuPickingBuffer;

    fn extract_component(data: QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        Some(GpuPickingBuffer(data.buffer.id()))
    }
}

#[derive(Component)]
pub struct GpuPickingBuffer(AssetId<ShaderStorageBuffer>);

#[derive(Default, Debug, Clone, ShaderType)]
pub struct GpuPickingData {
    pub cursor_coords: Vec2,
    pub depth: f32,
    pub stencil: u32,
    pub world_from_clip: Mat4,
    pub cell: IVec3,
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
                    storage_buffer::<GpuPickingData>(false),
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

#[derive(Debug, Hash, Default, PartialEq, Eq, Clone, RenderLabel)]
pub struct PickingNode;

impl render_graph::ViewNode for PickingNode {
    type ViewQuery = (&'static GpuPickingBuffer, &'static TerrainViewDepthTexture);

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        context: &mut RenderContext<'w>,
        (picking_buffer, depth): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let picking_pipeline = world.resource::<PickingPipeline>();
        let buffer = world.resource::<RenderAssets<GpuShaderStorageBuffer>>();

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(picking_pipeline.id) else {
            return Ok(());
        };

        let Some(buffer) = buffer.get(picking_buffer.0) else {
            return Ok(());
        };

        let bind_group = context.render_device().create_bind_group(
            None,
            &picking_pipeline.layout,
            &BindGroupEntries::sequential((
                buffer.buffer.as_entire_binding(),
                &depth.depth_view,
                &depth.stencil_view,
            )),
        );

        context.add_command_buffer_generation_task(move |device| {
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_pipeline(pipeline);
            pass.dispatch_workgroups(1, 1, 1);
            drop(pass);

            encoder.finish()
        });

        Ok(())
    }
}

pub struct TerrainPickingPlugin;

impl Plugin for TerrainPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            picking_system.after(TransformSystem::TransformPropagate),
        )
        .add_plugins(ExtractComponentPlugin::<PickingData>::default());

        app.sub_app_mut(RenderApp)
            .add_render_graph_node::<ViewNodeRunner<PickingNode>>(Core3d, PickingNode)
            .add_render_graph_edge(Core3d, TerrainPassNode, PickingNode);
    }
    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<PickingPipeline>();
    }
}
