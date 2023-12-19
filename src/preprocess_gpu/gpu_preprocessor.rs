use crate::preprocess::file_io::format_node_path;
use crate::preprocess::R16Image;
use crate::{
    preprocess_gpu::preprocessor::{PreprocessTask, PreprocessTaskType, Preprocessor},
    terrain::{Terrain, TerrainComponents},
    terrain_data::{gpu_node_atlas::GpuNodeAtlas, NodeCoordinate},
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{binding_types::*, *},
        renderer::{RenderDevice, RenderQueue},
        Extract,
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use itertools::Itertools;
use std::collections::VecDeque;
use std::{
    mem,
    path::Path,
    sync::{Arc, Mutex},
};

pub(crate) struct ProcessingTask {
    pub(crate) task: PreprocessTask,
    pub(crate) bind_group: Option<BindGroup>,
}

// Todo: this does not belong here
#[derive(Copy, Clone, Debug, Default, ShaderType)]
pub(crate) struct NodeMeta {
    pub(crate) node_coordinate: NodeCoordinate,
    #[size(16)]
    pub(crate) atlas_index: u32,
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

#[derive(Clone, Debug)]
pub(crate) struct ReadBackNode {
    pub(crate) data: Vec<u8>,
    pub(crate) texture_size: u32,
    pub(crate) meta: NodeMeta,
    pub(crate) save_to_disk: bool,
}

impl ReadBackNode {
    pub(crate) fn start_saving(self, path: String) -> Task<()> {
        AsyncComputeTaskPool::get().spawn(async move {
            if !self.save_to_disk {
                return ();
            };

            let image_data = self
                .data
                .chunks_exact(2)
                .map(|pixel| u16::from_le_bytes(pixel.try_into().unwrap()))
                .collect::<Vec<u16>>();

            let path = format_node_path(&path, &self.meta.node_coordinate);
            let path = Path::new(&path);
            let path = path.with_extension("png");
            let path = path.to_str().unwrap();

            let image =
                R16Image::from_raw(self.texture_size, self.texture_size, image_data).unwrap();

            image.save(path).unwrap();

            println!("Finished saving node: {path}");

            ()
        })
    }
}

pub(crate) fn create_split_tile_layout(device: &RenderDevice) -> BindGroupLayout {
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

pub(crate) fn create_stitch_node_layout(device: &RenderDevice) -> BindGroupLayout {
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
    pub(crate) read_back_tasks: Arc<Mutex<Vec<Task<Vec<ReadBackNode>>>>>,
}

impl GpuPreprocessor {
    pub(crate) fn new(preprocessor: &Preprocessor) -> Self {
        Self {
            ready_tasks: default(),
            processing_tasks: vec![],
            read_back_tasks: preprocessor.read_back_tasks.clone(),
        }
    }

    pub(crate) fn initialize(
        mut gpu_preprocessors: ResMut<TerrainComponents<GpuPreprocessor>>,
        terrain_query: Extract<Query<(Entity, &Preprocessor), Added<Terrain>>>,
    ) {
        for (terrain, preprocessor) in terrain_query.iter() {
            gpu_preprocessors.insert(terrain, GpuPreprocessor::new(preprocessor));
        }
    }

    pub(crate) fn extract(
        mut gpu_preprocessors: ResMut<TerrainComponents<GpuPreprocessor>>,
        terrain_query: Extract<Query<(Entity, &Preprocessor), With<Terrain>>>,
    ) {
        for (terrain, preprocessor) in terrain_query.iter() {
            let gpu_preprocessor = gpu_preprocessors.get_mut(&terrain).unwrap();

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
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        for terrain in terrain_query.iter() {
            let gpu_preprocessor = gpu_preprocessors.get_mut(&terrain).unwrap();

            let slots = 16;

            if !gpu_preprocessor.ready_tasks.is_empty() {
                for node_index in 0..slots {
                    if let Some(task) = gpu_preprocessor.ready_tasks.pop_back() {
                        let bind_group = match &task.task_type {
                            PreprocessTaskType::SplitTile { tile } => {
                                let tile = images.get(tile).unwrap();

                                let split_tile_data = SplitTileData {
                                    node_meta: task.node.clone(),
                                    node_index,
                                };

                                let mut split_tile_data_buffer =
                                    UniformBuffer::from(split_tile_data);
                                split_tile_data_buffer.write_buffer(&device, &queue);

                                Some(device.create_bind_group(
                                    "split_tile_bind_group",
                                    &create_split_tile_layout(&device),
                                    &BindGroupEntries::sequential((
                                        split_tile_data_buffer.binding().unwrap(),
                                        &tile.texture_view,
                                        &tile.sampler,
                                    )),
                                ))
                            }
                            PreprocessTaskType::Stitch { neighbour_nodes } => {
                                let stitch_node_data = StitchNodeData {
                                    node: task.node.clone(),
                                    neighbour_nodes: neighbour_nodes.clone(),
                                    node_index,
                                };

                                let mut stitch_node_data_buffer =
                                    UniformBuffer::from(stitch_node_data);
                                stitch_node_data_buffer.write_buffer(&device, &queue);

                                Some(device.create_bind_group(
                                    "stitch_node_bind_group",
                                    &create_stitch_node_layout(&device),
                                    &BindGroupEntries::single(
                                        stitch_node_data_buffer.binding().unwrap(),
                                    ),
                                ))
                            }
                            PreprocessTaskType::Downsample { parent_nodes } => {
                                let downsample_data = DownsampleData {
                                    node: task.node,
                                    parent_nodes: parent_nodes.clone(),
                                    node_index,
                                };

                                let mut downsample_data_buffer =
                                    UniformBuffer::from(downsample_data);
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

    pub(crate) fn cleanup(
        gpu_node_atlases: Res<TerrainComponents<GpuNodeAtlas>>,
        mut gpu_preprocessors: ResMut<TerrainComponents<GpuPreprocessor>>,
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        for terrain in terrain_query.iter() {
            let gpu_preprocessor = gpu_preprocessors.get_mut(&terrain).unwrap();
            let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();

            // Todo: start reading back all nodes, processed this frame
            if !gpu_preprocessor.processing_tasks.is_empty() {
                let attachment = &gpu_node_atlas.attachments[0];

                let tasks = mem::take(&mut gpu_preprocessor.processing_tasks);
                let tasks = tasks.into_iter().map(|task| task.task).collect_vec();

                gpu_preprocessor
                    .read_back_tasks
                    .lock()
                    .unwrap()
                    .push(attachment.start_reading_back_nodes(tasks));
            }
        }
    }
}
