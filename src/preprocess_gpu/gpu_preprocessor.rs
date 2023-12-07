use crate::preprocess::file_io::format_node_path;
use crate::preprocess::R16Image;
use crate::{
    preprocess_gpu::preprocessor::{PreprocessTask, PreprocessTaskType, Preprocessor},
    terrain::{Terrain, TerrainComponents},
    terrain_data::{gpu_node_atlas::GpuNodeAtlas, NodeCoordinate},
};
use bevy::utils::dbg;
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
use std::collections::VecDeque;
use std::{
    mem,
    path::Path,
    sync::{Arc, Mutex},
};

// Todo: this does not belong here
#[derive(Clone, Debug, ShaderType)]
pub(crate) struct NodeMeta {
    pub(crate) atlas_index: u32,
    pub(crate) _padding: u32,
    pub(crate) node_coordinate: NodeCoordinate,
}

#[derive(Clone, Debug)]
pub(crate) struct ReadBackNode {
    pub(crate) data: Vec<u8>,
    pub(crate) texture_size: u32,
    pub(crate) format: TextureFormat,
    pub(crate) meta: NodeMeta,
}

impl ReadBackNode {
    pub(crate) fn start_saving(self) -> Task<()> {
        AsyncComputeTaskPool::get().spawn(async move {
            let image_data = self
                .data
                .chunks_exact(2)
                .map(|pixel| u16::from_le_bytes(pixel.try_into().unwrap()))
                .collect::<Vec<u16>>();

            let path = format_node_path("assets/test", &self.meta.node_coordinate);
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

pub(crate) fn create_preprocess_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_2d(TextureSampleType::Float { filterable: true }), // tile
                sampler(SamplerBindingType::Filtering),                    // tile_sampler
                storage_buffer_read_only::<NodeMeta>(false),               // node_meta_list
            ),
        ),
    )
}

pub(crate) struct GpuPreprocessor {
    pub(crate) ready_tasks: VecDeque<PreprocessTask>,
    pub(crate) processing_tasks: Vec<PreprocessTask>,
    pub(crate) read_back_tasks: Arc<Mutex<Vec<Task<Vec<ReadBackNode>>>>>,
    pub(crate) preprocess_bind_group: Option<BindGroup>,
}

impl GpuPreprocessor {
    pub(crate) fn new(preprocessor: &Preprocessor) -> Self {
        Self {
            ready_tasks: default(),
            processing_tasks: vec![],
            read_back_tasks: preprocessor.read_back_tasks.clone(),
            preprocess_bind_group: None,
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

            if !gpu_preprocessor.ready_tasks.is_empty() {
                let task_type = gpu_preprocessor
                    .ready_tasks
                    .front()
                    .unwrap()
                    .task_type
                    .clone();
                let mut node_meta_list = vec![];

                // Todo: take slots amount of compatible ready task

                for _ in 0..4 {
                    if let Some(task) = gpu_preprocessor.ready_tasks.pop_back() {
                        if task.task_type == task_type {
                            node_meta_list.push(task.node.clone());
                            gpu_preprocessor.processing_tasks.push(task);
                        }
                    } else {
                        break;
                    }
                }

                let mut nodes_meta_buffer = StorageBuffer::from(node_meta_list);
                nodes_meta_buffer.write_buffer(&device, &queue);

                match &task_type {
                    PreprocessTaskType::SplitTile { tile } => {
                        let tile = images.get(tile).unwrap();

                        let preprocess_bind_group = device.create_bind_group(
                            "preprocess_bind_group",
                            &create_preprocess_layout(&device),
                            &BindGroupEntries::sequential((
                                &tile.texture_view,
                                &tile.sampler,
                                nodes_meta_buffer.binding().unwrap(),
                            )),
                        );

                        gpu_preprocessor.preprocess_bind_group = Some(preprocess_bind_group);
                    }
                    PreprocessTaskType::Stitch => {
                        todo!()
                    }
                    PreprocessTaskType::Downsample => {
                        todo!()
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

                gpu_preprocessor
                    .read_back_tasks
                    .lock()
                    .unwrap()
                    .push(attachment.start_reading_back_nodes(tasks));
            }
        }
    }
}
