use bevy::core_pipeline::core_3d::Camera3dDepthLoadOp;
use bevy::render::render_resource::{
    Extent3d, Texture, TextureDimension, TextureFormat, TextureView,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::{CachedTexture, DepthAttachment, TextureCache};
use bevy::{
    ecs::{entity::EntityHashSet, query::QueryItem},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, RenderSubGraph, ViewNode},
        render_phase::{
            CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem, PhaseItemExtraIndex,
            SortedPhaseItem, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::CachedRenderPipelineId,
        renderer::RenderContext,
        sync_world::{MainEntity, RenderEntity},
        view::ViewTarget,
        Extract,
    },
};
use std::ops::Range;
use wgpu::{
    CommandEncoderDescriptor, LoadOp, Operations, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, StoreOp, TextureDescriptor, TextureUsages,
};

// Todo: remove this
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
pub struct TerrainGraph;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct TerrainPass;

pub struct TerrainItem {
    pub representative_entity: (Entity, MainEntity),
    pub draw_function: DrawFunctionId,
    pub pipeline: CachedRenderPipelineId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    pub order: u32,
}

impl PhaseItem for TerrainItem {
    const AUTOMATIC_BATCHING: bool = false;

    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index
    }

    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for TerrainItem {
    type SortKey = u32;

    fn sort_key(&self) -> Self::SortKey {
        self.order
    }
}

impl CachedRenderPipelinePhaseItem for TerrainItem {
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_terrain_phases(
    cameras_3d: Extract<Query<(RenderEntity, &Camera), With<Camera3d>>>,
    mut live_entities: Local<EntityHashSet>,
    mut terrain_phases: ResMut<ViewSortedRenderPhases<TerrainItem>>,
) {
    live_entities.clear();

    for (entity, camera) in &cameras_3d {
        if !camera.is_active {
            continue;
        }

        terrain_phases.insert_or_clear(entity);
        live_entities.insert(entity);
    }

    terrain_phases.retain(|entity, _| live_entities.contains(entity));
}

#[derive(Component)]
pub struct TerrainViewDepthTexture {
    pub texture: Texture,
    attachment: DepthAttachment,
}

impl TerrainViewDepthTexture {
    pub fn new(texture: CachedTexture, clear_value: Option<f32>) -> Self {
        Self {
            texture: texture.texture,
            attachment: DepthAttachment::new(texture.default_view, clear_value),
        }
    }

    pub fn get_attachment(&self, store: StoreOp) -> RenderPassDepthStencilAttachment {
        self.attachment.get_attachment(store)
    }

    pub fn view(&self) -> &TextureView {
        &self.attachment.view
    }
}

pub fn prepare_terrain_depth_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    device: Res<RenderDevice>,
    views_3d: Query<(Entity, &ExtractedCamera, &Camera3d, &Msaa)>,
) {
    for (view, camera, camera_3d, msaa) in &views_3d {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        let descriptor = TextureDescriptor {
            label: Some("view_depth_texture"),
            size: Extent3d {
                depth_or_array_layers: 1,
                width: physical_target_size.x,
                height: physical_target_size.y,
            },
            mip_level_count: 1,
            sample_count: msaa.samples(),
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth24PlusStencil8,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let cached_texture = texture_cache.get(&device, descriptor);

        commands.entity(view).insert(TerrainViewDepthTexture::new(
            cached_texture,
            match camera_3d.depth_load_op {
                Camera3dDepthLoadOp::Clear(v) => Some(v),
                Camera3dDepthLoadOp::Load => None,
            },
        ));
    }
}

#[derive(Default)]
pub struct TerrainPassNode;
impl ViewNode for TerrainPassNode {
    type ViewQuery = (
        Entity,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static TerrainViewDepthTexture,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, camera, target, depth): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // dbg!("Running terrain pass!");

        let Some(terrain_phase) = world
            .get_resource::<ViewSortedRenderPhases<TerrainItem>>()
            .and_then(|phase| phase.get(&view))
        else {
            return Ok(());
        };

        if terrain_phase.items.is_empty() {
            return Ok(());
        }

        let color_attachments = [Some(target.get_color_attachment())];
        let depth_stencil_attachment = Some(RenderPassDepthStencilAttachment {
            view: depth.view(),
            depth_ops: Some(Operations {
                load: LoadOp::Clear(0.0), // Clear depth to 1.0
                store: StoreOp::Store,
            }),
            stencil_ops: Some(Operations {
                load: LoadOp::Clear(255), // Initialize stencil to 255 (lowest priority)
                store: StoreOp::Store,
            }),
        });

        let view_entity = graph.view_entity();
        render_context.add_command_buffer_generation_task(move |render_device| {
            // Command encoder setup
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor::default());

            // Render pass setup
            let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("terrain_pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            terrain_phase
                .render(&mut render_pass, world, view_entity)
                .unwrap();

            drop(render_pass);
            command_encoder.finish()
        });

        Ok(())
    }
}
