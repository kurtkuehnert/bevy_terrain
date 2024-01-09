use crate::{
    preprocess_gpu::preprocessor::{PreprocessTask, PreprocessTaskType, Preprocessor},
    terrain::{Terrain, TerrainComponents},
    terrain_data::{gpu_node_atlas::GpuNodeAtlas, node_atlas::AtlasNode},
    util::StaticBuffer,
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{binding_types::*, *},
        renderer::RenderDevice,
        Extract,
    },
};
use std::collections::VecDeque;

pub(crate) struct ProcessingTask {
    pub(crate) task: PreprocessTask,
    pub(crate) bind_group: Option<BindGroup>,
}

#[derive(Clone, Debug, ShaderType)]
pub(crate) struct SplitData {
    pub(crate) node: AtlasNode,
    pub(crate) node_index: u32,
}

#[derive(Clone, Debug, ShaderType)]
struct StitchData {
    node: AtlasNode,
    neighbour_nodes: [AtlasNode; 8],
    node_index: u32,
}

#[derive(Clone, Debug, ShaderType)]
struct DownsampleData {
    node: AtlasNode,
    child_nodes: [AtlasNode; 4],
    node_index: u32,
}

pub(crate) fn create_split_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<SplitData>(false), // split_tile_data
                texture_2d(TextureSampleType::Float { filterable: true }), // tile
                sampler(SamplerBindingType::Filtering), // tile_sampler
            ),
        ),
    )
}

pub(crate) fn create_stitch_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::single(ShaderStages::COMPUTE, uniform_buffer::<StitchData>(false)),
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
        images: Res<RenderAssets<Image>>,
        mut gpu_preprocessors: ResMut<TerrainComponents<GpuPreprocessor>>,
        mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        for terrain in terrain_query.iter() {
            let gpu_preprocessor = gpu_preprocessors.get_mut(&terrain).unwrap();
            let gpu_node_atlas = gpu_node_atlases.get_mut(&terrain).unwrap();

            gpu_preprocessor.processing_tasks.clear();

            while !gpu_preprocessor.ready_tasks.is_empty() {
                let task = gpu_preprocessor.ready_tasks.back().unwrap();
                let attachment =
                    &mut gpu_node_atlas.attachments[task.node.attachment_index as usize];

                if let Some(section_index) = attachment.reserve_write_slot(task.node) {
                    let task = gpu_preprocessor.ready_tasks.pop_back().unwrap();

                    let bind_group = match &task.task_type {
                        PreprocessTaskType::Split { tile } => {
                            let tile = images.get(tile).unwrap();

                            let split_buffer = StaticBuffer::create(
                                &device,
                                &SplitData {
                                    node: task.node.into(),
                                    node_index: section_index,
                                },
                                BufferUsages::UNIFORM,
                            );

                            Some(device.create_bind_group(
                                "split_bind_group",
                                &create_split_layout(&device),
                                &BindGroupEntries::sequential((
                                    &split_buffer,
                                    &tile.texture_view,
                                    &tile.sampler,
                                )),
                            ))
                        }
                        PreprocessTaskType::Stitch { neighbour_nodes } => {
                            let stitch_buffer = StaticBuffer::create(
                                &device,
                                &StitchData {
                                    node: task.node.into(),
                                    neighbour_nodes: *neighbour_nodes,
                                    node_index: section_index,
                                },
                                BufferUsages::UNIFORM,
                            );

                            Some(device.create_bind_group(
                                "stitch_bind_group",
                                &create_stitch_layout(&device),
                                &BindGroupEntries::single(&stitch_buffer),
                            ))
                        }
                        PreprocessTaskType::Downsample { child_nodes } => {
                            let downsample_buffer = StaticBuffer::create(
                                &device,
                                &DownsampleData {
                                    node: task.node.into(),
                                    child_nodes: *child_nodes,
                                    node_index: section_index,
                                },
                                BufferUsages::UNIFORM,
                            );

                            Some(device.create_bind_group(
                                "downsample_bind_group",
                                &create_downsample_layout(&device),
                                &BindGroupEntries::single(&downsample_buffer),
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
