use crate::{
    compute_phase::{
        ComputeFunction, ComputeFunctionId, ComputeFunctions, ComputePhaseItem, ViewComputePhases,
    },
    debug::DebugTerrain,
    render::{
        culling_bind_group::{create_culling_layout, CullingBindGroup},
        shaders::{PREPARE_INDIRECT_SHADER, REFINE_TILES_SHADER},
        terrain_bind_group::{create_terrain_layout, TerrainData},
        terrain_view_bind_group::{
            create_prepare_indirect_layout, create_refine_tiles_layout, TerrainViewData,
        },
    },
    terrain::{Terrain, TerrainComponents},
    terrain_view::{TerrainView, TerrainViewComponents},
    util::CollectArray,
};
use bevy::{
    ecs::entity::EntityHashSet,
    prelude::*,
    render::{
        render_graph::{self, RenderGraphContext, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        Extract,
    },
};
use itertools::Itertools;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount, EnumIter};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct TerrainComputeLabel;

type TerrainComputePipelineKey = (TerrainComputePipelineId, TerrainComputePipelineFlags);

#[derive(Copy, Clone, Hash, PartialEq, Eq, EnumIter, EnumCount)]
pub enum TerrainComputePipelineId {
    RefineTiles,
    PrepareRoot,
    PrepareNext,
    PrepareRender,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct TerrainComputePipelineFlags: u32 {
        const NONE               = 0;
        const SPHERICAL          = (1 << 0);
        const TEST1              = (1 << 1);
    }
}

impl TerrainComputePipelineFlags {
    pub fn from_debug(debug: &DebugTerrain) -> Self {
        let mut key = TerrainComputePipelineFlags::NONE;

        if debug.test1 {
            key |= TerrainComputePipelineFlags::TEST1;
        }

        key
    }

    pub fn shader_defs(&self) -> Vec<ShaderDefVal> {
        let mut shader_defs = Vec::new();

        if (self.bits() & TerrainComputePipelineFlags::SPHERICAL.bits()) != 0 {
            shader_defs.push("SPHERICAL".into());
        }
        if (self.bits() & TerrainComputePipelineFlags::TEST1.bits()) != 0 {
            shader_defs.push("TEST1".into());
        }

        shader_defs
    }
}

#[derive(Resource)]
pub struct TerrainComputePipelines {
    pub(crate) prepare_indirect_layout: BindGroupLayout,
    pub(crate) refine_tiles_layout: BindGroupLayout,
    culling_data_layout: BindGroupLayout,
    terrain_layout: BindGroupLayout,
    prepare_indirect_shader: Handle<Shader>,
    refine_tiles_shader: Handle<Shader>,
}

impl FromWorld for TerrainComputePipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let prepare_indirect_layout = create_prepare_indirect_layout(device);
        let refine_tiles_layout = create_refine_tiles_layout(device);
        let culling_data_layout = create_culling_layout(device);
        let terrain_layout = create_terrain_layout(device);

        let prepare_indirect_shader = asset_server.load(PREPARE_INDIRECT_SHADER);
        let refine_tiles_shader = asset_server.load(REFINE_TILES_SHADER);

        TerrainComputePipelines {
            prepare_indirect_layout,
            refine_tiles_layout,
            culling_data_layout,
            terrain_layout,
            prepare_indirect_shader,
            refine_tiles_shader,
        }
    }
}

impl SpecializedComputePipeline for TerrainComputePipelines {
    type Key = TerrainComputePipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let layout;
        let shader;
        let entry_point;

        let shader_defs = key.1.shader_defs();

        match key.0 {
            TerrainComputePipelineId::RefineTiles => {
                layout = vec![
                    self.culling_data_layout.clone(),
                    self.terrain_layout.clone(),
                    self.refine_tiles_layout.clone(),
                ];
                shader = self.refine_tiles_shader.clone();
                entry_point = "refine_tiles".into();
            }
            TerrainComputePipelineId::PrepareRoot => {
                layout = vec![
                    self.culling_data_layout.clone(),
                    self.terrain_layout.clone(),
                    self.refine_tiles_layout.clone(),
                    self.prepare_indirect_layout.clone(),
                ];
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_root".into();
            }
            TerrainComputePipelineId::PrepareNext => {
                layout = vec![
                    self.culling_data_layout.clone(),
                    self.terrain_layout.clone(),
                    self.refine_tiles_layout.clone(),
                    self.prepare_indirect_layout.clone(),
                ];
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_next".into();
            }
            TerrainComputePipelineId::PrepareRender => {
                layout = vec![
                    self.culling_data_layout.clone(),
                    self.terrain_layout.clone(),
                    self.refine_tiles_layout.clone(),
                    self.prepare_indirect_layout.clone(),
                ];
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_render".into();
            }
        }

        ComputePipelineDescriptor {
            label: Some("terrain_compute_pipeline".into()),
            layout,
            push_constant_ranges: default(),
            shader,
            shader_defs,
            entry_point,
        }
    }
}

pub struct TerrainComputeNode {
    view_query: QueryState<Entity, With<TerrainView>>,
}

impl FromWorld for TerrainComputeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            view_query: world.query_filtered(),
        }
    }
}

impl render_graph::Node for TerrainComputeNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), render_graph::NodeRunError> {
        let phases = world
            .get_resource::<ViewComputePhases<TerrainComputePhaseItem>>()
            .unwrap();

        let views = self.view_query.iter_manual(world).collect_vec();

        context.add_command_buffer_generation_task(move |device| {
            let mut command_encoder =
                device.create_command_encoder(&CommandEncoderDescriptor::default());
            let mut compute_pass =
                command_encoder.begin_compute_pass(&ComputePassDescriptor::default());

            for view in views {
                let phase = phases.get(&view).unwrap();
                phase.compute(&mut compute_pass, world, view);
            }

            drop(compute_pass);
            command_encoder.finish()
        });

        Ok(())
    }
}

pub(crate) struct TerrainComputePhaseItem {
    terrain: Entity,
    compute_function: ComputeFunctionId,
    pipelines: [CachedComputePipelineId; TerrainComputePipelineId::COUNT],
}

impl ComputePhaseItem for TerrainComputePhaseItem {
    fn entity(&self) -> Entity {
        self.terrain
    }

    fn compute_function(&self) -> ComputeFunctionId {
        self.compute_function
    }
}

#[derive(Default)]
pub(crate) struct TerrainComputeFunction;

impl ComputeFunction<TerrainComputePhaseItem> for TerrainComputeFunction {
    fn compute<'w>(
        &mut self,
        world: &'w World,
        pass: &mut ComputePass<'w>,
        view: Entity,
        item: &TerrainComputePhaseItem,
    ) {
        let pipeline_cache = world.resource::<PipelineCache>();
        let terrain_data = world.resource::<TerrainComponents<TerrainData>>();
        let terrain_view_data = world.resource::<TerrainViewComponents<TerrainViewData>>();
        let culling_bind_groups = world.resource::<TerrainViewComponents<CullingBindGroup>>();

        let debug = world.get_resource::<DebugTerrain>();

        if let Some(debug) = debug {
            if debug.freeze {
                return;
            }
        }

        let pipelines = match TerrainComputePipelineId::iter()
            .map(|id| pipeline_cache.get_compute_pipeline(item.pipelines[id as usize]))
            .collect::<Option<Vec<_>>>()
        {
            None => return, // some pipelines are not loaded yet
            Some(pipelines) => pipelines,
        };

        if let Some(terrain_data) = terrain_data.get(&item.terrain) {
            let view_data = terrain_view_data.get(&(item.terrain, view)).unwrap();
            let culling_bind_group = culling_bind_groups.get(&(item.terrain, view)).unwrap();

            pass.set_bind_group(0, culling_bind_group, &[]);
            pass.set_bind_group(1, &terrain_data.terrain_bind_group, &[]);
            pass.set_bind_group(2, &view_data.refine_tiles_bind_group, &[]);
            pass.set_bind_group(3, &view_data.prepare_indirect_bind_group, &[]);

            pass.set_pipeline(pipelines[TerrainComputePipelineId::PrepareRoot as usize]);
            pass.dispatch_workgroups(1, 1, 1);

            for _ in 0..view_data.refinement_count() {
                pass.set_pipeline(pipelines[TerrainComputePipelineId::RefineTiles as usize]);
                pass.dispatch_workgroups_indirect(&view_data.indirect_buffer, 0);

                pass.set_pipeline(pipelines[TerrainComputePipelineId::PrepareNext as usize]);
                pass.dispatch_workgroups(1, 1, 1);
            }

            pass.set_pipeline(pipelines[TerrainComputePipelineId::RefineTiles as usize]);
            pass.dispatch_workgroups_indirect(&view_data.indirect_buffer, 0);

            pass.set_pipeline(pipelines[TerrainComputePipelineId::PrepareRender as usize]);
            pass.dispatch_workgroups(1, 1, 1);
        }
    }
}

pub(crate) fn queue_terrain_compute(
    compute_functions: Res<ComputeFunctions<TerrainComputePhaseItem>>,
    debug: Option<Res<DebugTerrain>>,
    pipeline_cache: Res<PipelineCache>,
    compute_pipelines: ResMut<TerrainComputePipelines>,
    mut pipelines: ResMut<SpecializedComputePipelines<TerrainComputePipelines>>,
    mut terrain_compute_phases: ResMut<ViewComputePhases<TerrainComputePhaseItem>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    for view in view_query.iter() {
        let phase = terrain_compute_phases.get_mut(&view).unwrap();
        let compute_function = compute_functions
            .read()
            .get_id::<TerrainComputeFunction>()
            .unwrap();

        for terrain in terrain_query.iter() {
            let mut flags = TerrainComputePipelineFlags::NONE;

            #[cfg(feature = "spherical")]
            {
                flags |= TerrainComputePipelineFlags::SPHERICAL;
            }

            if let Some(debug) = &debug {
                flags |= TerrainComputePipelineFlags::from_debug(debug);
            }

            let pipelines = TerrainComputePipelineId::iter()
                .map(|id| pipelines.specialize(&pipeline_cache, &compute_pipelines, (id, flags)))
                .collect_array();

            phase.add(TerrainComputePhaseItem {
                terrain,
                compute_function,
                pipelines,
            });
        }
    }
}

pub(crate) fn extract_terrain_compute_phases(
    mut commands: Commands,
    mut terrain_compute_phases: ResMut<ViewComputePhases<TerrainComputePhaseItem>>,
    view_query: Extract<Query<Entity, With<TerrainView>>>,
    mut live_entities: Local<EntityHashSet>,
) {
    live_entities.clear();

    for view in &view_query {
        commands.get_or_spawn(view);

        terrain_compute_phases.insert_or_clear(view);

        live_entities.insert(view);
    }

    terrain_compute_phases.retain(|entity, _| live_entities.contains(entity));
}
