use crate::render::culling_bind_group::{create_culling_layout, CullingBindGroup};
use crate::render::terrain_bind_group::create_terrain_layout;
use crate::render::terrain_view_bind_group::{
    create_prepare_indirect_layout, create_refine_tiles_layout,
};
use crate::{
    debug::DebugTerrain,
    render::{GpuTerrainView, TerrainData},
    shaders::{PREPARE_PREPASS_SHADER, REFINE_TILES_SHADER},
    terrain::TerrainComponents,
    terrain_data::{GpuTileAtlas, GpuTileTree},
    terrain_view::TerrainViewComponents,
};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
    },
};
use std::ops::Deref;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct TilingPrepassLabel;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct TilingPrepassPipelineKey: u32 {
        const NONE           = 0;
        const REFINE_TILES   = 1 << 0;
        const PREPARE_ROOT   = 1 << 1;
        const PREPARE_NEXT   = 1 << 2;
        const PREPARE_RENDER = 1 << 3;
        const SPHERICAL      = 1 << 4;
        const HIGH_PRECISION = 1 << 5;
        const TEST1          = 1 << 6;
        const TEST2          = 1 << 7;
        const TEST3          = 1 << 8;
    }
}

impl TilingPrepassPipelineKey {
    pub fn from_debug(debug: &DebugTerrain) -> Self {
        let mut key = TilingPrepassPipelineKey::NONE;

        #[cfg(feature = "high_precision")]
        if debug.high_precision {
            key |= TilingPrepassPipelineKey::HIGH_PRECISION;
        }
        if debug.test1 {
            key |= TilingPrepassPipelineKey::TEST1;
        }
        if debug.test2 {
            key |= TilingPrepassPipelineKey::TEST2;
        }
        if debug.test3 {
            key |= TilingPrepassPipelineKey::TEST3;
        }

        key
    }

    pub fn shader_defs(&self) -> Vec<ShaderDefVal> {
        let mut shader_defs = Vec::new();

        if self.contains(TilingPrepassPipelineKey::SPHERICAL) {
            shader_defs.push("SPHERICAL".into());
        }
        #[cfg(feature = "high_precision")]
        if self.contains(TilingPrepassPipelineKey::HIGH_PRECISION) {
            shader_defs.push("HIGH_PRECISION".into());
        }
        if self.contains(TilingPrepassPipelineKey::TEST1) {
            shader_defs.push("TEST1".into());
        }
        if self.contains(TilingPrepassPipelineKey::TEST2) {
            shader_defs.push("TEST2".into());
        }
        if self.contains(TilingPrepassPipelineKey::TEST3) {
            shader_defs.push("TEST3".into());
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
        &'a ComputePipeline,
        &'a ComputePipeline,
        &'a ComputePipeline,
        &'a ComputePipeline,
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

        let prepare_indirect_layout = create_prepare_indirect_layout(device);
        let refine_tiles_layout = create_refine_tiles_layout(device);
        let culling_data_layout = create_culling_layout(device);
        let terrain_layout = create_terrain_layout(device);

        let prepare_prepass_shader = world.load_asset(PREPARE_PREPASS_SHADER);
        let refine_tiles_shader = world.load_asset(REFINE_TILES_SHADER);

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
            zero_initialize_workgroup_memory: false,
        }
    }
}

pub struct TilingPrepassNode;

impl render_graph::Node for TilingPrepassNode {
    fn run<'w>(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), render_graph::NodeRunError> {
        let prepass_items = world.resource::<TerrainViewComponents<TilingPrepassItem>>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let terrain_data = world.resource::<TerrainComponents<TerrainData>>();
        let gpu_terrain_views = world.resource::<TerrainViewComponents<GpuTerrainView>>();
        let culling_bind_groups = world.resource::<TerrainViewComponents<CullingBindGroup>>();
        let debug = world.get_resource::<DebugTerrain>();

        if debug.map(|debug| debug.freeze).unwrap_or(false) {
            return Ok(());
        }

        context.add_command_buffer_generation_task(move |device| {
            let mut command_encoder =
                device.create_command_encoder(&CommandEncoderDescriptor::default());
            let mut compute_pass =
                command_encoder.begin_compute_pass(&ComputePassDescriptor::default());

            for (&(terrain, view), prepass_item) in prepass_items.iter() {
                let Some((
                    refine_tiles_pipeline,
                    prepare_root_pipeline,
                    prepare_next_pipeline,
                    prepare_render_pipeline,
                )) = prepass_item.pipelines(pipeline_cache)
                else {
                    continue;
                };

                let culling_bind_group = culling_bind_groups.get(&(terrain, view)).unwrap();
                let terrain_data = terrain_data.get(&terrain).unwrap();
                let gpu_terrain_view = gpu_terrain_views.get(&(terrain, view)).unwrap();

                compute_pass.set_bind_group(0, culling_bind_group.deref(), &[]);
                compute_pass.set_bind_group(1, &terrain_data.terrain_bind_group, &[]);
                compute_pass.set_bind_group(2, &gpu_terrain_view.refine_tiles_bind_group, &[]);
                compute_pass.set_bind_group(3, &gpu_terrain_view.prepare_indirect_bind_group, &[]);

                compute_pass.set_pipeline(prepare_root_pipeline);
                compute_pass.dispatch_workgroups(1, 1, 1);

                for _ in 0..gpu_terrain_view.refinement_count() {
                    compute_pass.set_pipeline(refine_tiles_pipeline);
                    compute_pass.dispatch_workgroups_indirect(&gpu_terrain_view.indirect_buffer, 0);

                    compute_pass.set_pipeline(prepare_next_pipeline);
                    compute_pass.dispatch_workgroups(1, 1, 1);
                }

                compute_pass.set_pipeline(refine_tiles_pipeline);
                compute_pass.dispatch_workgroups_indirect(&gpu_terrain_view.indirect_buffer, 0);

                compute_pass.set_pipeline(prepare_render_pipeline);
                compute_pass.dispatch_workgroups(1, 1, 1);
            }

            drop(compute_pass);

            command_encoder.finish()
        });

        // unfortunately, we can not provide our own command encoder to the buffer readback
        // instead, it creates a new one and submits it to the queue
        let device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();

        for &(terrain, view) in prepass_items.keys() {
            let gpu_terrain_view = gpu_terrain_views.get(&(terrain, view)).unwrap();
            gpu_terrain_view.readback_view_height(&device, &queue);
        }

        Ok(())
    }
}

pub(crate) fn queue_tiling_prepass(
    debug: Option<Res<DebugTerrain>>,
    pipeline_cache: Res<PipelineCache>,
    prepass_pipelines: ResMut<TilingPrepassPipelines>,
    mut pipelines: ResMut<SpecializedComputePipelines<TilingPrepassPipelines>>,
    mut prepass_items: ResMut<TerrainViewComponents<TilingPrepassItem>>,
    gpu_tile_trees: Res<TerrainViewComponents<GpuTileTree>>,
    gpu_tile_atlases: Res<TerrainComponents<GpuTileAtlas>>,
) {
    for &(terrain, view) in gpu_tile_trees.keys() {
        let gpu_tile_atlas = gpu_tile_atlases.get(&terrain).unwrap();

        let mut key = TilingPrepassPipelineKey::NONE;

        if gpu_tile_atlas.is_spherical {
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
