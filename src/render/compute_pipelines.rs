use crate::{
    render::{compute_data::TerrainComputeData, culling::CullingBindGroup, layouts::*},
    GpuQuadtree, PersistentComponents,
};
use bevy::{
    ecs::system::{
        lifetimeless::{Read, SRes, SResMut},
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
    ActivateNodes,
    DeactivateNodes,
    BuildAreaList,
    BuildNodeList,
    BuildChunkList,
    PrepareAreaList,
    PrepareNodeList,
    PrepareRender,
}

pub struct TerrainComputePipelines {
    pub(crate) prepare_indirect_layout: BindGroupLayout,
    pub(crate) update_quadtree_layout: BindGroupLayout,
    pub(crate) build_node_list_layout: BindGroupLayout,
    pub(crate) cull_data_layout: BindGroupLayout,
    prepare_indirect_shader: Handle<Shader>,
    update_quadtree_shader: Handle<Shader>,
    build_node_list_shader: Handle<Shader>,
}

impl FromWorld for TerrainComputePipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let prepare_indirect_layout = device.create_bind_group_layout(&PREPARE_INDIRECT_LAYOUT);
        let update_quadtree_layout = device.create_bind_group_layout(&UPDATE_QUADTREE_LAYOUT);
        let build_node_list_layout = device.create_bind_group_layout(&BUILD_NODE_LIST_LAYOUT);
        let cull_data_layout = device.create_bind_group_layout(&CULL_DATA_LAYOUT);

        let prepare_indirect_shader =
            asset_server.load("../plugins/bevy_terrain/src/render/shaders/prepare_indirect.wgsl");
        let update_quadtree_shader =
            asset_server.load("../plugins/bevy_terrain/src/render/shaders/update_quadtree.wgsl");
        let build_node_list_shader =
            asset_server.load("../plugins/bevy_terrain/src/render/shaders/build_node_list.wgsl");

        TerrainComputePipelines {
            prepare_indirect_layout,
            update_quadtree_layout,
            build_node_list_layout,
            cull_data_layout,
            prepare_indirect_shader,
            update_quadtree_shader,
            build_node_list_shader,
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
            TerrainComputePipelineKey::ActivateNodes => {
                layout = Some(vec![self.update_quadtree_layout.clone()]);
                shader = self.update_quadtree_shader.clone();
                entry_point = "activate_nodes".into();
            }
            TerrainComputePipelineKey::DeactivateNodes => {
                layout = Some(vec![self.update_quadtree_layout.clone()]);
                shader = self.update_quadtree_shader.clone();
                entry_point = "deactivate_nodes".into();
            }
            TerrainComputePipelineKey::BuildAreaList => {
                layout = Some(vec![
                    self.build_node_list_layout.clone(),
                    self.cull_data_layout.clone(),
                ]);
                shader = self.build_node_list_shader.clone();
                entry_point = "build_area_list".into();
            }
            TerrainComputePipelineKey::BuildNodeList => {
                layout = Some(vec![
                    self.build_node_list_layout.clone(),
                    self.cull_data_layout.clone(),
                ]);
                shader = self.build_node_list_shader.clone();
                entry_point = "build_node_list".into();
            }
            TerrainComputePipelineKey::BuildChunkList => {
                layout = Some(vec![
                    self.build_node_list_layout.clone(),
                    self.cull_data_layout.clone(),
                ]);
                shader = self.build_node_list_shader.clone();
                entry_point = "build_chunk_list".into();
            }
            TerrainComputePipelineKey::PrepareAreaList => {
                layout = Some(vec![self.prepare_indirect_layout.clone()]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_area_list".into();
            }
            TerrainComputePipelineKey::PrepareNodeList => {
                layout = Some(vec![self.prepare_indirect_layout.clone()]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_node_list".into();
            }
            TerrainComputePipelineKey::PrepareRender => {
                layout = Some(vec![self.prepare_indirect_layout.clone()]);
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
    query: QueryState<(Entity, Read<CullingBindGroup>)>,
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
            query: world.query(),
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
        pass.set_bind_group(0, &gpu_quadtree.update_bind_group, &[]);
        pass.set_pipeline(pipelines[TerrainComputePipelineKey::ActivateNodes as usize]);
        pass.dispatch(gpu_quadtree.activation_count, 1, 1);
        pass.set_pipeline(pipelines[TerrainComputePipelineKey::DeactivateNodes as usize]);
        pass.dispatch(gpu_quadtree.deactivation_count, 1, 1);
    }

    fn build_node_list<'a>(
        pass: &mut ComputePass<'a>,
        pipelines: &'a Vec<&'a ComputePipeline>,
        bind_groups: &'a TerrainComputeData,
    ) {
        pass.set_bind_group(0, &bind_groups.prepare_indirect_bind_group, &[]);
        pass.set_pipeline(pipelines[TerrainComputePipelineKey::PrepareAreaList as usize]);
        pass.dispatch(1, 1, 1);

        pass.set_bind_group(0, &bind_groups.build_node_list_bind_groups[1], &[]);
        pass.set_pipeline(pipelines[TerrainComputePipelineKey::BuildAreaList as usize]);
        pass.dispatch_indirect(&bind_groups.indirect_buffer, 0);

        pass.set_bind_group(0, &bind_groups.prepare_indirect_bind_group, &[]);
        pass.set_pipeline(pipelines[TerrainComputePipelineKey::PrepareNodeList as usize]);
        pass.dispatch(1, 1, 1);

        let count = 7;

        for i in 0..count {
            pass.set_bind_group(0, &bind_groups.build_node_list_bind_groups[i % 2], &[]);
            pass.set_pipeline(pipelines[TerrainComputePipelineKey::BuildNodeList as usize]);
            pass.dispatch_indirect(&bind_groups.indirect_buffer, 0);

            pass.set_bind_group(0, &bind_groups.prepare_indirect_bind_group, &[]);
            pass.set_pipeline(pipelines[TerrainComputePipelineKey::PrepareNodeList as usize]);
            pass.dispatch(1, 1, 1);
        }

        pass.set_bind_group(0, &bind_groups.build_node_list_bind_groups[count % 2], &[]);
        pass.set_pipeline(pipelines[TerrainComputePipelineKey::BuildChunkList as usize]);
        pass.dispatch_indirect(&bind_groups.indirect_buffer, 0);
    }
}

impl render_graph::Node for TerrainComputeNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);

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
        let terrain_bind_groups = world.resource::<PersistentComponents<TerrainComputeData>>();
        let gpu_quadtrees = world.resource::<PersistentComponents<GpuQuadtree>>();

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

        for (entity, culling_bind_group) in self.query.iter_manual(world) {
            let gpu_quadtree = gpu_quadtrees.get(&entity).unwrap();
            let bind_groups = terrain_bind_groups.get(&entity).unwrap();

            pass.set_bind_group(1, &culling_bind_group.value, &[]);

            TerrainComputeNode::update_quadtree(pass, pipelines, gpu_quadtree);
            TerrainComputeNode::build_node_list(pass, pipelines, bind_groups);

            pass.set_bind_group(0, &bind_groups.prepare_indirect_bind_group, &[]);
            pass.set_pipeline(pipelines[TerrainComputePipelineKey::PrepareRender as usize]);
            pass.dispatch(1, 1, 1);
        }

        Ok(())
    }
}
