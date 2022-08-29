use crate::{
    render::{
        culling::CullingBindGroup,
        shaders::{PREPARE_INDIRECT_SHADER, TESSELATION_SHADER},
        terrain_data::terrain_bind_group_layout,
        terrain_view_data::TerrainViewData,
        CULL_DATA_LAYOUT, PREPARE_INDIRECT_LAYOUT, TESSELLATION_LAYOUT,
    },
    terrain::Terrain,
    DebugTerrain, TerrainComponents, TerrainData, TerrainPipelineConfig, TerrainView,
    TerrainViewComponents, TerrainViewConfig,
};
use bevy::{
    ecs::system::{
        lifetimeless::{SRes, SResMut},
        SystemState,
    },
    prelude::*,
    render::{
        render_graph::{self},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount, EnumIter};

type TerrainComputePipelineKey = (TerrainComputePipelineId, TerrainComputePipelineFlags);

#[derive(Copy, Clone, Hash, PartialEq, Eq, EnumIter, EnumCount)]
pub enum TerrainComputePipelineId {
    RefineTiles,
    PrepareTessellation,
    PrepareRefinement,
    PrepareRender,
}

bitflags::bitflags! {
#[repr(transparent)]
pub struct TerrainComputePipelineFlags: u32 {
    const NONE               = 0;
    const ADAPTIVE           = (1 << 0);
    const TEST               = (2 << 0);
}
}

impl TerrainComputePipelineFlags {
    pub fn from_debug(debug: &DebugTerrain) -> Self {
        let mut key = TerrainComputePipelineFlags::NONE;

        if debug.adaptive {
            key |= TerrainComputePipelineFlags::ADAPTIVE;
        }
        if debug.test1 {
            key |= TerrainComputePipelineFlags::TEST;
        }

        key
    }

    pub fn shader_defs(&self) -> Vec<String> {
        let mut shader_defs = Vec::new();

        if (self.bits & TerrainComputePipelineFlags::ADAPTIVE.bits) != 0 {
            shader_defs.push("ADAPTIVE".to_string());
        }
        if (self.bits & TerrainComputePipelineFlags::TEST.bits) != 0 {
            shader_defs.push("TEST".to_string());
        }

        shader_defs
    }
}

#[derive(Resource)]
pub struct TerrainComputePipelines {
    pub(crate) prepare_indirect_layout: BindGroupLayout,
    pub(crate) tessellation_layout: BindGroupLayout,
    pub(crate) cull_data_layout: BindGroupLayout,
    pub(crate) terrain_layout: BindGroupLayout,
    prepare_indirect_shader: Handle<Shader>,
    tessellation_shader: Handle<Shader>,
}

impl FromWorld for TerrainComputePipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let config = world.resource::<TerrainPipelineConfig>();

        let prepare_indirect_layout = device.create_bind_group_layout(&PREPARE_INDIRECT_LAYOUT);
        let tessellation_layout = device.create_bind_group_layout(&TESSELLATION_LAYOUT);
        let cull_data_layout = device.create_bind_group_layout(&CULL_DATA_LAYOUT);
        let terrain_layout = terrain_bind_group_layout(&device, config.attachment_count);

        let prepare_indirect_shader = PREPARE_INDIRECT_SHADER.typed();
        let tessellation_shader = TESSELATION_SHADER.typed();

        TerrainComputePipelines {
            prepare_indirect_layout,
            tessellation_layout,
            cull_data_layout,
            terrain_layout,
            prepare_indirect_shader,
            tessellation_shader,
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
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
                    self.terrain_layout.clone(),
                ]);
                shader = self.tessellation_shader.clone();
                entry_point = "refine_tiles".into();
            }
            TerrainComputePipelineId::PrepareTessellation => {
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
                    self.terrain_layout.clone(),
                    self.prepare_indirect_layout.clone(),
                ]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_tessellation".into();
            }
            TerrainComputePipelineId::PrepareRefinement => {
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
                    self.terrain_layout.clone(),
                    self.prepare_indirect_layout.clone(),
                ]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_refinement".into();
            }
            TerrainComputePipelineId::PrepareRender => {
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
                    self.terrain_layout.clone(),
                    self.prepare_indirect_layout.clone(),
                ]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_render".into();
            }
        }

        ComputePipelineDescriptor {
            label: Some("terrain_compute_pipeline".into()),
            layout,
            shader,
            shader_defs,
            entry_point,
        }
    }
}

pub struct TerrainComputeNode {
    terrain_query: QueryState<Entity, With<Terrain>>,
    view_query: QueryState<Entity, With<TerrainView>>,
    system_state: SystemState<(
        SResMut<PipelineCache>,
        SResMut<SpecializedComputePipelines<TerrainComputePipelines>>,
        SRes<TerrainComputePipelines>,
        Option<SRes<DebugTerrain>>,
    )>,
    pipelines: [CachedComputePipelineId; TerrainComputePipelineId::COUNT],
}

impl FromWorld for TerrainComputeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            terrain_query: world.query_filtered(),
            view_query: world.query_filtered(),
            system_state: SystemState::new(world),
            pipelines: [CachedComputePipelineId::INVALID; TerrainComputePipelineId::COUNT],
        }
    }
}

impl TerrainComputeNode {
    fn tessellate_terrain<'a>(
        pass: &mut ComputePass<'a>,
        pipelines: &'a Vec<&'a ComputePipeline>,
        view_data: &'a TerrainViewData,
        terrain_data: &'a TerrainData,
        culling_bind_group: &'a BindGroup,
        refinement_count: u32,
    ) {
        pass.set_bind_group(0, &view_data.tessellation_bind_group, &[]);
        pass.set_bind_group(1, culling_bind_group, &[]);
        pass.set_bind_group(2, &terrain_data.terrain_bind_group, &[]);
        pass.set_bind_group(3, &view_data.prepare_indirect_bind_group, &[]);

        pass.set_pipeline(pipelines[TerrainComputePipelineId::PrepareTessellation as usize]);
        pass.dispatch_workgroups(1, 1, 1);

        for _ in 0..refinement_count {
            pass.set_pipeline(pipelines[TerrainComputePipelineId::RefineTiles as usize]);
            pass.dispatch_workgroups_indirect(&view_data.indirect_buffer, 0);

            pass.set_pipeline(pipelines[TerrainComputePipelineId::PrepareRefinement as usize]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        pass.set_pipeline(pipelines[TerrainComputePipelineId::RefineTiles as usize]);
        pass.dispatch_workgroups_indirect(&view_data.indirect_buffer, 0);

        pass.set_pipeline(pipelines[TerrainComputePipelineId::PrepareRender as usize]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}

impl render_graph::Node for TerrainComputeNode {
    fn update(&mut self, world: &mut World) {
        self.terrain_query.update_archetypes(world);
        self.view_query.update_archetypes(world);

        let (mut pipeline_cache, mut pipelines, pipeline, debug) = self.system_state.get_mut(world);

        let mut flags = TerrainComputePipelineFlags::NONE;

        if let Some(debug) = &debug {
            flags |= TerrainComputePipelineFlags::from_debug(debug);
        } else {
            flags |= TerrainComputePipelineFlags::ADAPTIVE
        }

        for id in TerrainComputePipelineId::iter() {
            self.pipelines[id as usize] =
                pipelines.specialize(&mut pipeline_cache, &pipeline, (id, flags));
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let view_configs = world.resource::<TerrainViewComponents<TerrainViewConfig>>();
        let terrain_data = world.resource::<TerrainComponents<TerrainData>>();
        let terrain_view_data = world.resource::<TerrainViewComponents<TerrainViewData>>();
        let culling_bind_groups = world.resource::<TerrainViewComponents<CullingBindGroup>>();

        let pipelines = &match TerrainComputePipelineId::iter()
            .map(|key| pipeline_cache.get_compute_pipeline(self.pipelines[key as usize]))
            .collect::<Option<Vec<_>>>()
        {
            None => return Ok(()), // some pipelines are not loaded yet
            Some(pipelines) => pipelines,
        };

        let pass = &mut context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        for terrain in self.terrain_query.iter_manual(world) {
            let terrain_data = terrain_data.get(&terrain).unwrap();
            for view in self.view_query.iter_manual(world) {
                let view_config = view_configs.get(&(terrain, view)).unwrap();
                let view_data = terrain_view_data.get(&(terrain, view)).unwrap();
                let culling_bind_group = culling_bind_groups.get(&(terrain, view)).unwrap();

                TerrainComputeNode::tessellate_terrain(
                    pass,
                    pipelines,
                    view_data,
                    terrain_data,
                    &culling_bind_group.value,
                    view_config.refinement_count,
                );
            }
        }

        Ok(())
    }
}
