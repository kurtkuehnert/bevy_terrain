use crate::preprocess::R16Image;
use crate::preprocess_gpu::preprocess_data::{PreprocessData, PREPROCESS_LAYOUT};
use crate::preprocess_gpu::shaders::SPLIT_TILE_SHADER;
use crate::terrain::{Terrain, TerrainComponents};
use crate::terrain_data::gpu_node_atlas::{align_byte_size, GpuNodeAtlas, ATTACHMENT_LAYOUT};
use async_channel;
use bevy::render::render_asset::RenderAssets;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::{
    prelude::*,
    render::{
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

        let attachment_layout = device.create_bind_group_layout(&ATTACHMENT_LAYOUT);
        let preprocess_layout = device.create_bind_group_layout(&PREPROCESS_LAYOUT);

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
        let preprocess_data = world.resource::<TerrainComponents<PreprocessData>>();
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

                // decide which nodes to process this frame
                let atlas_indices = 0..1;

                attachment.copy_atlas_to_rw_nodes(
                    &mut context.command_encoder(),
                    images,
                    atlas_indices.clone(),
                );

                {
                    // dispatch shader
                    let pass = &mut context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor::default());
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
                        1,
                    );
                }

                attachment.copy_rw_nodes_to_atlas(
                    &mut context.command_encoder(),
                    images,
                    atlas_indices,
                );

                attachment.read_back_node(
                    context.command_encoder(),
                    images,
                    preprocess_data.read_back_buffer.as_ref().unwrap(),
                    0,
                );
            }
        }

        Ok(())
    }
}

pub(crate) fn save_node(read_back_buffer: Buffer) {
    let width = 512;
    let height = 512;
    let pixel_size = 2;

    let finish = async move {
        let (tx, rx) = async_channel::bounded(1);
        let buffer_slice = read_back_buffer.slice(..);
        // The polling for this map call is done every frame when the command queue is submitted.
        buffer_slice.map_async(MapMode::Read, move |result| {
            let err = result.err();
            if err.is_some() {
                panic!("{}", err.unwrap().to_string());
            }
            tx.try_send(()).unwrap();
        });
        rx.recv().await.unwrap();
        let data = buffer_slice.get_mapped_range();
        // we immediately move the data to CPU memory to avoid holding the mapped view for long
        let mut result = Vec::from(&*data);
        drop(data);
        drop(read_back_buffer);

        if result.len() != ((width * height) as usize * pixel_size) {
            // Our buffer has been padded because we needed to align to a multiple of 256.
            // We remove this padding here
            let initial_row_bytes = width as usize * pixel_size;
            let buffered_row_bytes = align_byte_size(width * pixel_size as u32) as usize;

            let mut take_offset = buffered_row_bytes;
            let mut place_offset = initial_row_bytes;
            for _ in 1..height {
                result.copy_within(take_offset..take_offset + buffered_row_bytes, place_offset);
                take_offset += buffered_row_bytes;
                place_offset += initial_row_bytes;
            }
            result.truncate(initial_row_bytes * height as usize);
        }

        let result: Vec<u16> = result
            .chunks_exact(2)
            .map(|pixel| u16::from_le_bytes(pixel.try_into().unwrap()))
            .collect();

        let image = R16Image::from_raw(width, height, result).unwrap();

        image.save("test.png").unwrap();

        dbg!("node data has been retreived from the GPU");
    };

    AsyncComputeTaskPool::get().spawn(finish).detach();
}
