use crate::{
    preprocess_gpu::{
        gpu_preprocessor::{create_preprocess_layout, GpuPreprocessor},
        shaders::SPLIT_TILE_SHADER,
    },
    terrain::{Terrain, TerrainComponents},
    terrain_data::{
        gpu_atlas_attachment::{create_attachment_layout, GpuAtlasAttachment},
        gpu_node_atlas::GpuNodeAtlas,
    },
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

type TerrainPreprocessPipelineKey = TerrainPreprocessPipelineId;

#[derive(Copy, Clone, Hash, PartialEq, Eq, EnumIter)]
pub enum TerrainPreprocessPipelineId {
    SplitTile,
}

#[derive(Resource)]
pub struct TerrainPreprocessPipelines {
    attachment_layout: BindGroupLayout,
    preprocess_layout: BindGroupLayout,
    pipelines: Vec<CachedComputePipelineId>,
}

impl FromWorld for TerrainPreprocessPipelines {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let attachment_layout = create_attachment_layout(&device);
        let preprocess_layout = create_preprocess_layout(&device);

        let mut preprocess_pipelines = TerrainPreprocessPipelines {
            attachment_layout,
            preprocess_layout,
            pipelines: vec![],
        };

        world.resource_scope(|world: &mut World,mut pipelines: Mut<SpecializedComputePipelines<TerrainPreprocessPipelines>>| {
            let pipeline_cache = world.resource::<PipelineCache>();
            for id in TerrainPreprocessPipelineId::iter() {
                preprocess_pipelines.pipelines.push(pipelines.specialize(&pipeline_cache, &preprocess_pipelines, id));
            }
        });

        preprocess_pipelines
    }
}

impl SpecializedComputePipeline for TerrainPreprocessPipelines {
    type Key = TerrainPreprocessPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let layout;
        let shader;
        let entry_point;

        let shader_defs = vec![];

        match key {
            TerrainPreprocessPipelineId::SplitTile => {
                layout = vec![
                    self.attachment_layout.clone(),
                    self.preprocess_layout.clone(),
                ];
                shader = SPLIT_TILE_SHADER;
                entry_point = "split_tile".into();
            }
        }

        ComputePipelineDescriptor {
            label: Some("terrain_preprocess_pipeline".into()),
            layout,
            push_constant_ranges: default(),
            shader,
            shader_defs,
            entry_point,
        }
    }
}

pub struct TerrainPreprocessNode {
    terrain_query: QueryState<Entity, With<Terrain>>,
}

impl TerrainPreprocessNode {
    fn split_tile(
        command_encoder: &mut CommandEncoder,
        pipelines: &[&ComputePipeline],
        attachment: &GpuAtlasAttachment,
        preprocess_data: &GpuPreprocessor,
        node_count: u32,
    ) {
        // dispatch shader
        let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_pipeline(pipelines[TerrainPreprocessPipelineId::SplitTile as usize]);
        pass.set_bind_group(0, &attachment.bind_group, &[]);
        pass.set_bind_group(
            1,
            preprocess_data.preprocess_bind_group.as_ref().unwrap(),
            &[],
        );
        pass.dispatch_workgroups(
            attachment.workgroup_count.x,
            attachment.workgroup_count.y,
            node_count,
        );
    }
}

impl FromWorld for TerrainPreprocessNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            terrain_query: world.query_filtered(),
        }
    }
}

impl render_graph::Node for TerrainPreprocessNode {
    fn update(&mut self, world: &mut World) {
        self.terrain_query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let preprocess_pipelines = world.resource::<TerrainPreprocessPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let preprocess_data = world.resource::<TerrainComponents<GpuPreprocessor>>();
        let gpu_node_atlases = world.resource::<TerrainComponents<GpuNodeAtlas>>();

        let images = world.resource::<RenderAssets<Image>>();

        let pipelines = &match TerrainPreprocessPipelineId::iter()
            .map(|id| {
                pipeline_cache.get_compute_pipeline(preprocess_pipelines.pipelines[id as usize])
            })
            .collect::<Option<Vec<_>>>()
        {
            None => return Ok(()), // some pipelines are not loaded yet
            Some(pipelines) => pipelines,
        };

        for terrain in self.terrain_query.iter_manual(world) {
            let preprocess_data = preprocess_data.get(&terrain).unwrap();
            let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();

            if preprocess_data.is_ready {
                dbg!("running Pipeline");

                let attachment = &gpu_node_atlas.attachments[0];

                let nodes = &preprocess_data.affected_nodes[0..4];

                attachment.copy_nodes_to_write_section(context.command_encoder(), images, nodes);

                TerrainPreprocessNode::split_tile(
                    context.command_encoder(),
                    pipelines,
                    attachment,
                    preprocess_data,
                    nodes.len() as u32,
                );

                attachment.copy_nodes_from_write_section(context.command_encoder(), images, nodes);

                attachment.download_nodes(context.command_encoder(), images, nodes);
            }
        }

        Ok(())
    }
}
