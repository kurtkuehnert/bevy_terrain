use crate::shaders::DEPTH_COPY_SHADER;
use bevy::{
    core_pipeline::{
        core_3d::CORE_3D_DEPTH_FORMAT, fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::{entity::EntityHashSet, query::QueryItem},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_phase::{
            CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem, PhaseItemExtraIndex,
            SortedPhaseItem, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{binding_types::texture_depth_2d_multisampled, *},
        renderer::{RenderContext, RenderDevice},
        sync_world::{MainEntity, RenderEntity},
        texture::{CachedTexture, TextureCache},
        view::{ViewDepthTexture, ViewTarget},
        Extract,
    },
};
use std::ops::Range;

pub(crate) const TERRAIN_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32FloatStencil8;

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
        u32::MAX - self.order
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
    texture: Texture,
    pub view: TextureView,
    pub depth_view: TextureView,
    pub stencil_view: TextureView,
}

impl TerrainViewDepthTexture {
    pub fn new(texture: CachedTexture) -> Self {
        let depth_view = texture.texture.create_view(&TextureViewDescriptor {
            aspect: TextureAspect::DepthOnly,
            ..default()
        });
        let stencil_view = texture.texture.create_view(&TextureViewDescriptor {
            aspect: TextureAspect::StencilOnly,
            ..default()
        });

        Self {
            texture: texture.texture,
            view: texture.default_view,
            depth_view,
            stencil_view,
        }
    }

    pub fn get_attachment(&self) -> RenderPassDepthStencilAttachment {
        RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(0.0), // Clear depth
                store: StoreOp::Store,
            }),
            stencil_ops: Some(Operations {
                load: LoadOp::Clear(0), // Initialize stencil to 0 (lowest priority)
                store: StoreOp::Store,
            }),
        }
    }
}

pub fn prepare_terrain_depth_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    device: Res<RenderDevice>,
    views_3d: Query<(Entity, &ExtractedCamera, &Msaa)>,
) {
    for (view, camera, msaa) in &views_3d {
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
            format: TERRAIN_DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let cached_texture = texture_cache.get(&device, descriptor);

        commands
            .entity(view)
            .insert(TerrainViewDepthTexture::new(cached_texture));
    }
}

#[derive(Resource)]
pub struct DepthCopyPipeline {
    layout: BindGroupLayout,
    id: CachedRenderPipelineId,
}

impl FromWorld for DepthCopyPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let layout = device.create_bind_group_layout(
            None,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (texture_depth_2d_multisampled(),),
            ),
        );

        let id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: None,
            layout: vec![layout.clone()],
            push_constant_ranges: Vec::new(),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: world.load_asset(DEPTH_COPY_SHADER),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![],
            }),
            primitive: Default::default(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: 4, // Todo: specialize per camera ...
                ..Default::default()
            },
            zero_initialize_workgroup_memory: false,
        });

        Self { layout, id }
    }
}

#[derive(Debug, Hash, Default, PartialEq, Eq, Clone, RenderLabel)]
pub struct TerrainPass;

impl ViewNode for TerrainPass {
    type ViewQuery = (
        Entity,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static TerrainViewDepthTexture,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, camera, target, depth, terrain_depth): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let depth_copy_pipeline = world.resource::<DepthCopyPipeline>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(depth_copy_pipeline.id) else {
            return Ok(());
        };

        let Some(terrain_phase) = world
            .get_resource::<ViewSortedRenderPhases<TerrainItem>>()
            .and_then(|phase| phase.get(&view))
        else {
            return Ok(());
        };

        if terrain_phase.items.is_empty() {
            return Ok(());
        }

        // Todo: prepare this in a separate system
        let terrain_depth_view = terrain_depth.texture.create_view(&TextureViewDescriptor {
            aspect: TextureAspect::DepthOnly,
            ..default()
        });
        let depth_copy_bind_group = device.create_bind_group(
            None,
            &depth_copy_pipeline.layout,
            &BindGroupEntries::single(&terrain_depth_view),
        );

        // call this here, otherwise the order between passes is incorrect
        let color_attachments = [Some(target.get_color_attachment())];
        let terrain_depth_stencil_attachment = Some(terrain_depth.get_attachment());
        let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

        render_context.add_command_buffer_generation_task(move |device| {
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

            let pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("terrain_pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: terrain_depth_stencil_attachment,
                ..default()
            });
            let mut pass = TrackedRenderPass::new(&device, pass);

            if let Some(viewport) = camera.viewport.as_ref() {
                pass.set_camera_viewport(viewport);
            }

            terrain_phase.render(&mut pass, world, view).unwrap();
            drop(pass);

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                depth_stencil_attachment,
                ..default()
            });
            pass.set_bind_group(0, &depth_copy_bind_group, &[]);
            pass.set_pipeline(pipeline);
            pass.draw(0..3, 0..1);
            drop(pass);

            encoder.finish()
        });

        Ok(())
    }
}
