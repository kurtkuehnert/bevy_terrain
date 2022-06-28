use crate::{
    render::{culling::CullingBindGroup, layouts::*, terrain_view_data::TerrainViewData},
    terrain::Terrain,
    GpuQuadtree, TerrainView, TerrainViewComponents, TerrainViewConfig, PREPARE_INDIRECT_HANDLE,
    TESSELATION_HANDLE, UPDATE_QUADTREE_HANDLE,
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

#[derive(Copy, Clone, Hash, PartialEq, Eq, EnumIter, EnumCount)]
pub enum TerrainComputePipelineKey {
    UpdateQuadtree,
    SelectCoarsestPatches,
    RefinePatches,
    SelectFinestPatches,
    PrepareTessellation,
    PrepareRefinement,
    PrepareRender,
}

pub struct TerrainComputePipelines {
    pub(crate) prepare_indirect_layout: BindGroupLayout,
    pub(crate) update_quadtree_layout: BindGroupLayout,
    pub(crate) tessellation_layout: BindGroupLayout,
    pub(crate) cull_data_layout: BindGroupLayout,
    prepare_indirect_shader: Handle<Shader>,
    update_quadtree_shader: Handle<Shader>,
    tessellation_shader: Handle<Shader>,
}

impl FromWorld for TerrainComputePipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let prepare_indirect_layout = device.create_bind_group_layout(&PREPARE_INDIRECT_LAYOUT);
        let update_quadtree_layout = device.create_bind_group_layout(&UPDATE_QUADTREE_LAYOUT);
        let tessellation_layout = device.create_bind_group_layout(&TESSELLATION_LAYOUT);
        let cull_data_layout = device.create_bind_group_layout(&CULL_DATA_LAYOUT);

        let prepare_indirect_shader = PREPARE_INDIRECT_HANDLE.typed();
        let update_quadtree_shader = UPDATE_QUADTREE_HANDLE.typed();
        let tessellation_shader = TESSELATION_HANDLE.typed();

        TerrainComputePipelines {
            prepare_indirect_layout,
            update_quadtree_layout,
            tessellation_layout,
            cull_data_layout,
            prepare_indirect_shader,
            update_quadtree_shader,
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

        match key {
            TerrainComputePipelineKey::UpdateQuadtree => {
                layout = Some(vec![self.update_quadtree_layout.clone()]);
                shader = self.update_quadtree_shader.clone();
                entry_point = "update_quadtree".into();
            }
            TerrainComputePipelineKey::SelectCoarsestPatches => {
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
                ]);
                shader = self.tessellation_shader.clone();
                entry_point = "select_coarsest_patches".into();
            }
            TerrainComputePipelineKey::RefinePatches => {
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
                ]);
                shader = self.tessellation_shader.clone();
                entry_point = "refine_patches".into();
            }
            TerrainComputePipelineKey::SelectFinestPatches => {
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
                ]);
                shader = self.tessellation_shader.clone();
                entry_point = "select_finest_patches".into();
            }
            TerrainComputePipelineKey::PrepareTessellation => {
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
                    self.prepare_indirect_layout.clone(),
                ]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_tessellation".into();
            }
            TerrainComputePipelineKey::PrepareRefinement => {
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
                    self.prepare_indirect_layout.clone(),
                ]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_refinement".into();
            }
            TerrainComputePipelineKey::PrepareRender => {
                layout = Some(vec![
                    self.tessellation_layout.clone(),
                    self.cull_data_layout.clone(),
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
            shader_defs: Vec::new(),
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
    )>,
    pipelines: [CachedComputePipelineId; TerrainComputePipelineKey::COUNT],
}

impl FromWorld for TerrainComputeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            terrain_query: world.query_filtered(),
            view_query: world.query_filtered(),
            system_state: SystemState::new(world),
            pipelines: [CachedComputePipelineId::INVALID; TerrainComputePipelineKey::COUNT],
        }
    }
}

impl TerrainComputeNode {
    fn update_quadtree<'a>(
        pass: &mut ComputePass<'a>,
        pipelines: &'a Vec<&'a ComputePipeline>,
        gpu_quadtree: &'a GpuQuadtree,
    ) {
        if gpu_quadtree.node_updates.len() != 0 {
            pass.set_bind_group(0, &gpu_quadtree.update_bind_group, &[]);
            pass.set_pipeline(pipelines[TerrainComputePipelineKey::UpdateQuadtree as usize]);
            pass.dispatch(gpu_quadtree.node_updates.len() as u32, 1, 1);
        }
    }

    fn tessellate_terrain<'a>(
        pass: &mut ComputePass<'a>,
        pipelines: &'a Vec<&'a ComputePipeline>,
        data: &'a TerrainViewData,
        culling_bind_group: &'a BindGroup,
        refinement_count: u32,
    ) {
        pass.set_bind_group(0, &data.tessellation_bind_group, &[]);
        pass.set_bind_group(1, culling_bind_group, &[]);
        pass.set_bind_group(2, &data.prepare_indirect_bind_group, &[]);

        pass.set_pipeline(pipelines[TerrainComputePipelineKey::PrepareTessellation as usize]);
        pass.dispatch(1, 1, 1);

        pass.set_pipeline(pipelines[TerrainComputePipelineKey::SelectCoarsestPatches as usize]);
        pass.dispatch_indirect(&data.indirect_buffer, 0);

        pass.set_pipeline(pipelines[TerrainComputePipelineKey::PrepareRefinement as usize]);
        pass.dispatch(1, 1, 1);

        for _ in 0..refinement_count {
            pass.set_pipeline(pipelines[TerrainComputePipelineKey::RefinePatches as usize]);
            pass.dispatch_indirect(&data.indirect_buffer, 0);

            pass.set_pipeline(pipelines[TerrainComputePipelineKey::PrepareRefinement as usize]);
            pass.dispatch(1, 1, 1);
        }

        pass.set_pipeline(pipelines[TerrainComputePipelineKey::SelectFinestPatches as usize]);
        pass.dispatch_indirect(&data.indirect_buffer, 0);

        pass.set_pipeline(pipelines[TerrainComputePipelineKey::PrepareRender as usize]);
        pass.dispatch(1, 1, 1);
    }
}

impl render_graph::Node for TerrainComputeNode {
    fn update(&mut self, world: &mut World) {
        self.terrain_query.update_archetypes(world);
        self.view_query.update_archetypes(world);

        let (mut pipeline_cache, mut pipelines, pipeline) = self.system_state.get_mut(world);

        for key in TerrainComputePipelineKey::iter() {
            self.pipelines[key as usize] =
                pipelines.specialize(&mut pipeline_cache, &pipeline, key);
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
        let terrain_view_data = world.resource::<TerrainViewComponents<TerrainViewData>>();
        let gpu_quadtrees = world.resource::<TerrainViewComponents<GpuQuadtree>>();
        let culling_bind_groups = world.resource::<TerrainViewComponents<CullingBindGroup>>();

        let pipelines = &match TerrainComputePipelineKey::iter()
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
            for view in self.view_query.iter_manual(world) {
                let view_config = view_configs.get(&(terrain, view)).unwrap();
                let data = terrain_view_data.get(&(terrain, view)).unwrap();
                let gpu_quadtree = gpu_quadtrees.get(&(terrain, view)).unwrap();
                let culling_bind_group = culling_bind_groups.get(&(terrain, view)).unwrap();

                TerrainComputeNode::update_quadtree(pass, pipelines, gpu_quadtree);
                TerrainComputeNode::tessellate_terrain(
                    pass,
                    pipelines,
                    data,
                    &culling_bind_group.value,
                    view_config.refinement_count,
                );
            }
        }

        Ok(())
    }
}
