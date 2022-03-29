use crate::render::terrain_data::GpuTerrainData;
use crate::{
    node_atlas::GpuNodeAtlas,
    render::{culling::CullingBindGroup, layouts::*, terrain_data::TerrainData},
};
use bevy::asset::load_internal_asset;
use bevy::ecs::system::lifetimeless::{SRes, SResMut};
use bevy::ecs::system::SystemState;
use bevy::reflect::TypeUuid;
use bevy::{
    ecs::system::lifetimeless::Read,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};
use itertools::Itertools;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount, EnumIter};

#[derive(Copy, Clone, Hash, PartialEq, Eq, EnumIter, EnumCount)]
pub enum TerrainPipelineKey {
    UpdateQuadtree,
    BuildChunkMaps,
    BuildAreaList,
    BuildNodeList,
    BuildChunkList,
    BuildPatchList,
    PrepareAreaList,
    PrepareNodeList,
    PreparePatchList,
    PrepareRender,
}

pub struct TerrainComputePipelines {
    pub(crate) prepare_indirect_layout: BindGroupLayout,
    pub(crate) update_quadtree_layout: BindGroupLayout,
    pub(crate) build_node_list_layout: BindGroupLayout,
    pub(crate) build_patch_list_layout: BindGroupLayout,
    pub(crate) build_chunk_maps_layout: BindGroupLayout,
    pub(crate) cull_data_layout: BindGroupLayout,
    prepare_indirect_shader: Handle<Shader>,
    update_quadtree_shader: Handle<Shader>,
    build_node_list_shader: Handle<Shader>,
    build_patch_list_shader: Handle<Shader>,
    build_chunk_maps_shader: Handle<Shader>,
}

impl FromWorld for TerrainComputePipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let update_quadtree_layout = device.create_bind_group_layout(&UPDATE_QUADTREE_LAYOUT);
        let build_node_list_layout = device.create_bind_group_layout(&BUILD_NODE_LIST_LAYOUT);
        let prepare_indirect_layout = device.create_bind_group_layout(&PREPARE_INDIRECT_LAYOUT);
        let build_patch_list_layout = device.create_bind_group_layout(&BUILD_PATCH_LIST_LAYOUT);
        let build_chunk_maps_layout = device.create_bind_group_layout(&BUILD_CHUNK_MAPS_LAYOUT);
        let cull_data_layout = device.create_bind_group_layout(&CULL_DATA_LAYOUT);

        let prepare_indirect_shader =
            asset_server.load("../plugins/bevy_terrain/src/render/shaders/prepare_indirect.wgsl");
        let update_quadtree_shader =
            asset_server.load("../plugins/bevy_terrain/src/render/shaders/update_quadtree.wgsl");
        let build_node_list_shader =
            asset_server.load("../plugins/bevy_terrain/src/render/shaders/build_node_list.wgsl");
        let build_patch_list_shader =
            asset_server.load("../plugins/bevy_terrain/src/render/shaders/build_patch_list.wgsl");
        let build_chunk_maps_shader =
            asset_server.load("../plugins/bevy_terrain/src/render/shaders/build_chunk_maps.wgsl");

        TerrainComputePipelines {
            prepare_indirect_layout,
            update_quadtree_layout,
            build_node_list_layout,
            build_patch_list_layout,
            build_chunk_maps_layout,
            cull_data_layout,
            prepare_indirect_shader,
            update_quadtree_shader,
            build_node_list_shader,
            build_patch_list_shader,
            build_chunk_maps_shader,
        }
    }
}

impl SpecializedComputePipeline for TerrainComputePipelines {
    type Key = TerrainPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let layout;
        let shader;
        let entry_point;

        match key {
            TerrainPipelineKey::UpdateQuadtree => {
                layout = Some(vec![self.update_quadtree_layout.clone()]);
                shader = self.update_quadtree_shader.clone();
                entry_point = "update_quadtree".into();
            }
            TerrainPipelineKey::BuildChunkMaps => {
                layout = Some(vec![self.build_chunk_maps_layout.clone()]);
                shader = self.build_chunk_maps_shader.clone_weak();
                entry_point = "build_chunk_maps".into();
            }
            TerrainPipelineKey::BuildAreaList => {
                layout = Some(vec![self.build_node_list_layout.clone()]);
                shader = self.build_node_list_shader.clone();
                entry_point = "build_area_list".into();
            }
            TerrainPipelineKey::BuildNodeList => {
                layout = Some(vec![self.build_node_list_layout.clone()]);
                shader = self.build_node_list_shader.clone();
                entry_point = "build_node_list".into();
            }
            TerrainPipelineKey::BuildChunkList => {
                layout = Some(vec![self.build_node_list_layout.clone()]);
                shader = self.build_node_list_shader.clone();
                entry_point = "build_chunk_list".into();
            }
            TerrainPipelineKey::BuildPatchList => {
                layout = Some(vec![
                    self.build_patch_list_layout.clone(),
                    self.cull_data_layout.clone(),
                ]);
                shader = self.build_patch_list_shader.clone();
                entry_point = "build_patch_list".into();
            }
            TerrainPipelineKey::PrepareAreaList => {
                layout = Some(vec![self.prepare_indirect_layout.clone()]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_area_list".into();
            }
            TerrainPipelineKey::PrepareNodeList => {
                layout = Some(vec![self.prepare_indirect_layout.clone()]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_node_list".into();
            }
            TerrainPipelineKey::PreparePatchList => {
                layout = Some(vec![self.prepare_indirect_layout.clone()]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_patch_list".into();
            }
            TerrainPipelineKey::PrepareRender => {
                layout = Some(vec![self.prepare_indirect_layout.clone()]);
                shader = self.prepare_indirect_shader.clone();
                entry_point = "prepare_render".into();
            }
        }

        ComputePipelineDescriptor {
            label: None,
            layout,
            shader,
            shader_defs: Vec::new(),
            entry_point,
        }
    }
}

pub struct TerrainComputeNode {
    query: QueryState<(
        Option<Read<GpuNodeAtlas>>,
        Read<Handle<TerrainData>>,
        Read<CullingBindGroup>,
    )>,
    system_state: SystemState<(
        SResMut<PipelineCache>,
        SResMut<SpecializedComputePipelines<TerrainComputePipelines>>,
        SRes<TerrainComputePipelines>,
    )>,
    pipelines: [CachedComputePipelineId; TerrainPipelineKey::COUNT],
}

impl FromWorld for TerrainComputeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query(),
            system_state: SystemState::new(world),
            pipelines: [CachedComputePipelineId::INVALID; TerrainPipelineKey::COUNT],
        }
    }
}

impl TerrainComputeNode {
    fn update_quadtree<'a>(
        pass: &mut ComputePass<'a>,
        pipelines: &'a Vec<&'a ComputePipeline>,
        gpu_node_atlas: Option<&GpuNodeAtlas>,
        gpu_terrain_data: &'a GpuTerrainData,
    ) {
        pass.set_pipeline(pipelines[TerrainPipelineKey::UpdateQuadtree as usize]);

        if let Some(gpu_node_atlas) = gpu_node_atlas {
            for (count, bind_group) in gpu_node_atlas
                .node_update_counts
                .iter()
                .zip(&gpu_terrain_data.update_quadtree_bind_groups)
            {
                pass.set_bind_group(0, bind_group, &[]);
                pass.dispatch(*count, 1, 1);
            }
        }
    }

    fn build_node_list<'a>(
        pass: &mut ComputePass<'a>,
        pipelines: &'a Vec<&'a ComputePipeline>,
        gpu_terrain_data: &'a GpuTerrainData,
    ) {
        pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
        pass.set_pipeline(pipelines[TerrainPipelineKey::PrepareAreaList as usize]);
        pass.dispatch(1, 1, 1);

        pass.set_bind_group(0, &gpu_terrain_data.build_node_list_bind_groups[1], &[]);
        pass.set_pipeline(pipelines[TerrainPipelineKey::BuildAreaList as usize]);
        pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

        pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
        pass.set_pipeline(pipelines[TerrainPipelineKey::PrepareNodeList as usize]);
        pass.dispatch(1, 1, 1);

        let count = gpu_terrain_data.config.lod_count as usize - 1;

        for i in 0..count {
            pass.set_bind_group(0, &gpu_terrain_data.build_node_list_bind_groups[i % 2], &[]);
            pass.set_pipeline(pipelines[TerrainPipelineKey::BuildNodeList as usize]);
            pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

            pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
            pass.set_pipeline(pipelines[TerrainPipelineKey::PrepareNodeList as usize]);
            pass.dispatch(1, 1, 1);
        }

        pass.set_bind_group(
            0,
            &gpu_terrain_data.build_node_list_bind_groups[count % 2],
            &[],
        );
        pass.set_pipeline(pipelines[TerrainPipelineKey::BuildChunkList as usize]);
        pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);
    }

    fn build_patch_list<'a>(
        pass: &mut ComputePass<'a>,
        pipelines: &'a Vec<&'a ComputePipeline>,
        gpu_terrain_data: &'a GpuTerrainData,
    ) {
        pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
        pass.set_pipeline(pipelines[TerrainPipelineKey::PreparePatchList as usize]);
        pass.dispatch(1, 1, 1);

        pass.set_bind_group(0, &gpu_terrain_data.build_chunk_maps_bind_group, &[]);
        pass.set_pipeline(pipelines[TerrainPipelineKey::BuildChunkMaps as usize]);
        pass.dispatch(
            gpu_terrain_data.config.chunk_count.x * gpu_terrain_data.config.chunk_count.y,
            1,
            1,
        );

        pass.set_bind_group(0, &gpu_terrain_data.build_patch_list_bind_group, &[]);
        pass.set_pipeline(pipelines[TerrainPipelineKey::BuildPatchList as usize]);
        pass.dispatch_indirect(&gpu_terrain_data.indirect_buffer, 0);

        pass.set_bind_group(0, &gpu_terrain_data.prepare_indirect_bind_group, &[]);
        pass.set_pipeline(pipelines[TerrainPipelineKey::PrepareRender as usize]);
        pass.dispatch(1, 1, 1);
    }
}

impl render_graph::Node for TerrainComputeNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);

        let (mut pipeline_cache, mut pipelines, pipeline) = self.system_state.get_mut(world);

        for key in TerrainPipelineKey::iter() {
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
        let terrain_data = world.resource::<RenderAssets<TerrainData>>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let pipelines = &match TerrainPipelineKey::iter()
            .map(|key| pipeline_cache.get_compute_pipeline(self.pipelines[key as usize]))
            .collect::<Option<Vec<_>>>()
        {
            None => return Ok(()), // some pipelines are not loaded yet
            Some(pipelines) => pipelines,
        };

        let mut pass = &mut context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        for (gpu_node_atlas, handle, culling_bind_group) in self.query.iter_manual(world) {
            let gpu_terrain_data = terrain_data.get(handle).unwrap();

            pass.set_bind_group(1, &culling_bind_group.value, &[]);

            TerrainComputeNode::update_quadtree(pass, pipelines, gpu_node_atlas, gpu_terrain_data);
            TerrainComputeNode::build_node_list(pass, pipelines, gpu_terrain_data);
            TerrainComputeNode::build_patch_list(pass, pipelines, gpu_terrain_data);
        }

        Ok(())
    }
}
