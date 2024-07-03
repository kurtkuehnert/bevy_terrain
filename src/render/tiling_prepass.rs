use crate::{
    debug::DebugTerrain,
    render::{
        culling_bind_group::{create_culling_layout, CullingBindGroup},
        terrain_bind_group::{create_terrain_layout, TerrainData},
        terrain_view_bind_group::{
            create_prepare_indirect_layout, create_refine_tiles_layout, TerrainViewData,
        },
    },
    shaders::{PREPARE_PREPASS_SHADER, REFINE_TILES_SHADER},
    terrain::{Terrain, TerrainComponents, TerrainConfig},
    terrain_view::{TerrainView, TerrainViewComponents},
};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self, NodeRunError, RenderGraphContext, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};
use itertools::Itertools;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct TilingPrepassLabel;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct TilingPrepassPipelineKey: u32 {
        const NONE           = 1 << 0;
        const REFINE_TILES   = 1 << 1;
        const PREPARE_ROOT   = 1 << 2;
        const PREPARE_NEXT   = 1 << 3;
        const PREPARE_RENDER = 1 << 4;
        const SPHERICAL      = 1 << 5;
        const TEST1          = 1 << 6;
    }
}

impl TilingPrepassPipelineKey {
    pub fn from_debug(debug: &DebugTerrain) -> Self {
        let mut key = TilingPrepassPipelineKey::NONE;

        if debug.test1 {
            key |= TilingPrepassPipelineKey::TEST1;
        }

        key
    }

    pub fn shader_defs(&self) -> Vec<ShaderDefVal> {
        let mut shader_defs = Vec::new();

        if self.contains(TilingPrepassPipelineKey::SPHERICAL) {
            shader_defs.push("SPHERICAL".into());
        }
        if self.contains(TilingPrepassPipelineKey::TEST1) {
            shader_defs.push("TEST1".into());
        }

        shader_defs
    }
}

pub(crate) struct TilingPrepassItem {
    refine_tiles_pipeline: CachedComputePipelineId,
    prepare_root_pipeline: CachedComputePipelineId,
    prepare_next_pipeline: CachedComputePipelineId,
    prepare_render_pipeline: CachedComputePipelineId,
}

impl TilingPrepassItem {
    fn pipelines<'a>(
        &'a self,
        pipeline_cache: &'a PipelineCache,
    ) -> Option<(
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
        &ComputePipeline,
    )> {
        Some((
            pipeline_cache.get_compute_pipeline(self.refine_tiles_pipeline)?,
            pipeline_cache.get_compute_pipeline(self.prepare_root_pipeline)?,
            pipeline_cache.get_compute_pipeline(self.prepare_next_pipeline)?,
            pipeline_cache.get_compute_pipeline(self.prepare_render_pipeline)?,
        ))
    }
}

#[derive(Resource)]
pub struct TilingPrepassPipelines {
    pub(crate) prepare_indirect_layout: BindGroupLayout,
    pub(crate) refine_tiles_layout: BindGroupLayout,
    culling_data_layout: BindGroupLayout,
    terrain_layout: BindGroupLayout,
    prepare_prepass_shader: Handle<Shader>,
    refine_tiles_shader: Handle<Shader>,
}

impl FromWorld for TilingPrepassPipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let prepare_indirect_layout = create_prepare_indirect_layout(device);
        let refine_tiles_layout = create_refine_tiles_layout(device);
        let culling_data_layout = create_culling_layout(device);
        let terrain_layout = create_terrain_layout(device);

        let prepare_prepass_shader = asset_server.load(PREPARE_PREPASS_SHADER);
        let refine_tiles_shader = asset_server.load(REFINE_TILES_SHADER);

        TilingPrepassPipelines {
            prepare_indirect_layout,
            refine_tiles_layout,
            culling_data_layout,
            terrain_layout,
            prepare_prepass_shader,
            refine_tiles_shader,
        }
    }
}

impl SpecializedComputePipeline for TilingPrepassPipelines {
    type Key = TilingPrepassPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let mut layout = default();
        let mut shader = default();
        let mut entry_point = default();

        let shader_defs = key.shader_defs();

        if key.contains(TilingPrepassPipelineKey::REFINE_TILES) {
            layout = vec![
                self.culling_data_layout.clone(),
                self.terrain_layout.clone(),
                self.refine_tiles_layout.clone(),
            ];
            shader = self.refine_tiles_shader.clone();
            entry_point = "refine_tiles".into();
        }
        if key.contains(TilingPrepassPipelineKey::PREPARE_ROOT) {
            layout = vec![
                self.culling_data_layout.clone(),
                self.terrain_layout.clone(),
                self.refine_tiles_layout.clone(),
                self.prepare_indirect_layout.clone(),
            ];
            shader = self.prepare_prepass_shader.clone();
            entry_point = "prepare_root".into();
        }
        if key.contains(TilingPrepassPipelineKey::PREPARE_NEXT) {
            layout = vec![
                self.culling_data_layout.clone(),
                self.terrain_layout.clone(),
                self.refine_tiles_layout.clone(),
                self.prepare_indirect_layout.clone(),
            ];
            shader = self.prepare_prepass_shader.clone();
            entry_point = "prepare_next".into();
        }
        if key.contains(TilingPrepassPipelineKey::PREPARE_RENDER) {
            layout = vec![
                self.culling_data_layout.clone(),
                self.terrain_layout.clone(),
                self.refine_tiles_layout.clone(),
                self.prepare_indirect_layout.clone(),
            ];
            shader = self.prepare_prepass_shader.clone();
            entry_point = "prepare_render".into();
        }

        ComputePipelineDescriptor {
            label: Some("tiling_prepass_pipeline".into()),
            layout,
            push_constant_ranges: default(),
            shader,
            shader_defs,
            entry_point,
        }
    }
}

pub struct TilingPrepassNode {
    view_query: QueryState<Entity, With<TerrainView>>,
    terrain_query: QueryState<Entity, With<Terrain>>,
}

impl FromWorld for TilingPrepassNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: world.query_filtered(),
            terrain_query: world.query_filtered(),
        }
    }
}

impl render_graph::Node for TilingPrepassNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
        self.terrain_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let prepass_items = world.resource::<TerrainViewComponents<TilingPrepassItem>>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let terrain_data = world.resource::<TerrainComponents<TerrainData>>();
        let terrain_view_data = world.resource::<TerrainViewComponents<TerrainViewData>>();
        let culling_bind_groups = world.resource::<TerrainViewComponents<CullingBindGroup>>();
        let debug = world.get_resource::<DebugTerrain>();

        if debug.map(|debug| debug.freeze).unwrap_or(false) {
            return Ok(());
        }

        let views = self.view_query.iter_manual(world).collect_vec();
        let terrains = self.terrain_query.iter_manual(world).collect_vec();

        context.add_command_buffer_generation_task(move |device| {
            let mut command_encoder =
                device.create_command_encoder(&CommandEncoderDescriptor::default());
            let mut compute_pass =
                command_encoder.begin_compute_pass(&ComputePassDescriptor::default());

            for &view in &views {
                for &terrain in &terrains {
                    let item = prepass_items.get(&(terrain, view)).unwrap();

                    let Some((
                        refine_tiles_pipeline,
                        prepare_root_pipeline,
                        prepare_next_pipeline,
                        prepare_render_pipeline,
                    )) = item.pipelines(pipeline_cache)
                    else {
                        continue;
                    };

                    let terrain_data = terrain_data.get(&terrain).unwrap();
                    let view_data = terrain_view_data.get(&(terrain, view)).unwrap();
                    let culling_bind_group = culling_bind_groups.get(&(terrain, view)).unwrap();

                    compute_pass.set_bind_group(0, culling_bind_group, &[]);
                    compute_pass.set_bind_group(1, &terrain_data.terrain_bind_group, &[]);
                    compute_pass.set_bind_group(2, &view_data.refine_tiles_bind_group, &[]);
                    compute_pass.set_bind_group(3, &view_data.prepare_indirect_bind_group, &[]);

                    compute_pass.set_pipeline(prepare_root_pipeline);
                    compute_pass.dispatch_workgroups(1, 1, 1);

                    for _ in 0..view_data.refinement_count() {
                        compute_pass.set_pipeline(refine_tiles_pipeline);
                        compute_pass.dispatch_workgroups_indirect(&view_data.indirect_buffer, 0);

                        compute_pass.set_pipeline(prepare_next_pipeline);
                        compute_pass.dispatch_workgroups(1, 1, 1);
                    }

                    compute_pass.set_pipeline(refine_tiles_pipeline);
                    compute_pass.dispatch_workgroups_indirect(&view_data.indirect_buffer, 0);

                    compute_pass.set_pipeline(prepare_render_pipeline);
                    compute_pass.dispatch_workgroups(1, 1, 1);
                }
            }

            drop(compute_pass);
            command_encoder.finish()
        });

        Ok(())
    }
}

pub(crate) fn queue_tiling_prepass(
    debug: Option<Res<DebugTerrain>>,
    pipeline_cache: Res<PipelineCache>,
    prepass_pipelines: ResMut<TilingPrepassPipelines>,
    mut pipelines: ResMut<SpecializedComputePipelines<TilingPrepassPipelines>>,
    mut prepass_items: ResMut<TerrainViewComponents<TilingPrepassItem>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<(Entity, &TerrainConfig), With<Terrain>>,
) {
    for view in view_query.iter() {
        for (terrain, config) in terrain_query.iter() {
            let mut key = TilingPrepassPipelineKey::NONE;

            if config.model.spherical {
                key |= TilingPrepassPipelineKey::SPHERICAL;
            }

            if let Some(debug) = &debug {
                key |= TilingPrepassPipelineKey::from_debug(debug);
            }

            let refine_tiles_pipeline = pipelines.specialize(
                &pipeline_cache,
                &prepass_pipelines,
                key | TilingPrepassPipelineKey::REFINE_TILES,
            );
            let prepare_root_pipeline = pipelines.specialize(
                &pipeline_cache,
                &prepass_pipelines,
                key | TilingPrepassPipelineKey::PREPARE_ROOT,
            );
            let prepare_next_pipeline = pipelines.specialize(
                &pipeline_cache,
                &prepass_pipelines,
                key | TilingPrepassPipelineKey::PREPARE_NEXT,
            );
            let prepare_render_pipeline = pipelines.specialize(
                &pipeline_cache,
                &prepass_pipelines,
                key | TilingPrepassPipelineKey::PREPARE_RENDER,
            );

            prepass_items.insert(
                (terrain, view),
                TilingPrepassItem {
                    refine_tiles_pipeline,
                    prepare_root_pipeline,
                    prepare_next_pipeline,
                    prepare_render_pipeline,
                },
            );
        }
    }
}
