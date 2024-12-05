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
        view::{ViewDepthTexture, ViewTarget},
        Extract,
    },
};
use std::ops::Range;
use wgpu::{CommandEncoderDescriptor, RenderPassDescriptor, StoreOp};

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
    pub priority: u8,
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
    type SortKey = u8;

    fn sort_key(&self) -> Self::SortKey {
        self.priority
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

#[derive(Default)]
pub struct TerrainPassNode;
impl ViewNode for TerrainPassNode {
    type ViewQuery = (
        Entity,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewDepthTexture,
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
        let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

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
