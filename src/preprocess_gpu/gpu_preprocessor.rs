use crate::{
    preprocess_gpu::preprocessor::{PreprocessTask, PreprocessTaskType, Preprocessor},
    terrain::{Terrain, TerrainComponents},
    terrain_data::{gpu_node_atlas::GpuNodeAtlas, node_atlas::NodeMeta},
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{binding_types::*, *},
        renderer::{RenderDevice, RenderQueue},
        Extract,
    },
};
use std::collections::VecDeque;

pub(crate) struct ProcessingTask {
    pub(crate) task: PreprocessTask,
    pub(crate) bind_group: Option<BindGroup>,
}

#[derive(Clone, Debug, ShaderType)]
pub(crate) struct SplitTileData {
    pub(crate) node_meta: NodeMeta,
    pub(crate) node_index: u32,
}

#[derive(Clone, Debug, ShaderType)]
struct StitchNodeData {
    node: NodeMeta,
    neighbour_nodes: [NodeMeta; 8],
    node_index: u32,
}

#[derive(Clone, Debug, ShaderType)]
struct DownsampleData {
    node: NodeMeta,
    parent_nodes: [NodeMeta; 4],
    node_index: u32,
}

pub(crate) fn create_split_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<SplitTileData>(false), // split_tile_data
                texture_2d(TextureSampleType::Float { filterable: true }), // tile
                sampler(SamplerBindingType::Filtering), // tile_sampler
            ),
        ),
    )
}

pub(crate) fn create_stitch_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::single(
            ShaderStages::COMPUTE,
            uniform_buffer::<StitchNodeData>(false),
        ),
    )
}

pub(crate) fn create_downsample_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::single(
            ShaderStages::COMPUTE,
            uniform_buffer::<DownsampleData>(false),
        ),
    )
}

pub(crate) struct GpuPreprocessor {
    pub(crate) ready_tasks: VecDeque<PreprocessTask>,
    pub(crate) processing_tasks: Vec<ProcessingTask>,
}

impl GpuPreprocessor {
    pub(crate) fn new() -> Self {
        Self {
            ready_tasks: default(),
            processing_tasks: vec![],
        }
    }

    pub(crate) fn initialize(
        mut gpu_preprocessors: ResMut<TerrainComponents<GpuPreprocessor>>,
        terrain_query: Extract<Query<Entity, Added<Terrain>>>,
    ) {
        for terrain in terrain_query.iter() {
            gpu_preprocessors.insert(terrain, GpuPreprocessor::new());
        }
    }

    pub(crate) fn extract(
        mut gpu_preprocessors: ResMut<TerrainComponents<GpuPreprocessor>>,
        terrain_query: Extract<Query<(Entity, &Preprocessor), With<Terrain>>>,
    ) {
        for (terrain, preprocessor) in terrain_query.iter() {
            let gpu_preprocessor = gpu_preprocessors.get_mut(&terrain).unwrap();

            // Todo: mem take using &mut world?
            gpu_preprocessor
                .ready_tasks
                .extend(preprocessor.ready_tasks.clone().into_iter());
        }
    }

    pub(crate) fn prepare(
        device: Res<RenderDevice>,
        queue: Res<RenderQueue>,
        images: Res<RenderAssets<Image>>,
        mut gpu_preprocessors: ResMut<TerrainComponents<GpuPreprocessor>>,
        mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        for terrain in terrain_query.iter() {
            let gpu_preprocessor = gpu_preprocessors.get_mut(&terrain).unwrap();
            let gpu_node_atlas = gpu_node_atlases.get_mut(&terrain).unwrap();

            gpu_preprocessor.processing_tasks.clear();

            let attachment = &mut gpu_node_atlas.attachments[0];

            while !gpu_preprocessor.ready_tasks.is_empty() {
                let node_meta = gpu_preprocessor.ready_tasks.back().unwrap().node;

                if let Some(section_index) = attachment.reserve_write_slot(node_meta) {
                    let task = gpu_preprocessor.ready_tasks.pop_back().unwrap();

                    let bind_group = match &task.task_type {
                        PreprocessTaskType::Split { tile } => {
                            let tile = images.get(tile).unwrap();

                            let split_tile_data = SplitTileData {
                                node_meta: task.node,
                                node_index: section_index,
                            };

                            let mut split_tile_data_buffer = UniformBuffer::from(split_tile_data);
                            split_tile_data_buffer.write_buffer(&device, &queue);

                            Some(device.create_bind_group(
                                "split_tile_bind_group",
                                &create_split_layout(&device),
                                &BindGroupEntries::sequential((
                                    split_tile_data_buffer.binding().unwrap(),
                                    &tile.texture_view,
                                    &tile.sampler,
                                )),
                            ))
                        }
                        PreprocessTaskType::Stitch { neighbour_nodes } => {
                            let stitch_node_data = StitchNodeData {
                                node: task.node,
                                neighbour_nodes: *neighbour_nodes,
                                node_index: section_index,
                            };

                            let mut stitch_node_data_buffer = UniformBuffer::from(stitch_node_data);
                            stitch_node_data_buffer.write_buffer(&device, &queue);

                            Some(device.create_bind_group(
                                "stitch_node_bind_group",
                                &create_stitch_layout(&device),
                                &BindGroupEntries::single(
                                    stitch_node_data_buffer.binding().unwrap(),
                                ),
                            ))
                        }
                        PreprocessTaskType::Downsample { parent_nodes } => {
                            let downsample_data = DownsampleData {
                                node: task.node,
                                parent_nodes: *parent_nodes,
                                node_index: section_index,
                            };

                            let mut downsample_data_buffer = UniformBuffer::from(downsample_data);
                            downsample_data_buffer.write_buffer(&device, &queue);

                            Some(device.create_bind_group(
                                "downsample_bind_group",
                                &create_downsample_layout(&device),
                                &BindGroupEntries::single(
                                    downsample_data_buffer.binding().unwrap(),
                                ),
                            ))
                        }
                        _ => break,
                    };

                    gpu_preprocessor
                        .processing_tasks
                        .push(ProcessingTask { task, bind_group });
                } else {
                    break;
                }
            }
        }
    }
}
