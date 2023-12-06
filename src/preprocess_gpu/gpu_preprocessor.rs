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
    tasks::{futures_lite::future, Task},
};
use std::{
    mem,
    sync::{Arc, Mutex},
};

// Todo: this does not belong here
#[derive(Clone, Debug, ShaderType)]
pub(crate) struct NodeMeta {
    pub(crate) atlas_index: u32,
    pub(crate) _padding: u32,
    pub(crate) node_coordinate: NodeCoordinate,
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
    pub(crate) ready_tasks: Vec<PreprocessTask>,
    pub(crate) processing_tasks: Vec<PreprocessTask>,
    pub(crate) saving_task: Option<Task<Vec<PreprocessTask>>>,
    pub(crate) finished_tasks: Arc<Mutex<Vec<PreprocessTask>>>,
    pub(crate) preprocess_bind_group: Option<BindGroup>,
}

impl GpuPreprocessor {
    pub(crate) fn new(preprocessor: &Preprocessor) -> Self {
        Self {
            ready_tasks: vec![],
            processing_tasks: vec![],
            saving_task: None,
            finished_tasks: preprocessor.finished_tasks.clone(),
            preprocess_bind_group: None,
        }
    }

    pub(crate) fn update(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        images: &RenderAssets<Image>,
        gpu_node_atlas: &GpuNodeAtlas,
    ) {
        if let Some(task) = &mut self.saving_task {
            if let Some(mut finished_tasks) = future::block_on(future::poll_once(task)) {
                self.finished_tasks
                    .lock()
                    .unwrap()
                    .append(&mut finished_tasks);
                self.saving_task = None;
            }
        }

        if !self.processing_tasks.is_empty() {
            let attachment = &gpu_node_atlas.attachments[0];

            let tasks = mem::take(&mut self.processing_tasks);

            self.saving_task = Some(attachment.save_nodes(tasks));
        }

        if !self.ready_tasks.is_empty() {
            let node_meta_list = self
                .ready_tasks
                .iter()
                .map(|task| task.node.clone())
                .collect::<Vec<_>>();
            let mut nodes_meta_buffer = StorageBuffer::from(node_meta_list);
            nodes_meta_buffer.write_buffer(device, queue);

            let task = self.ready_tasks.remove(0);

            match &task.task_type {
                PreprocessTaskType::SplitTile { tile } => {
                    let tile = images.get(tile).unwrap();

                    let preprocess_bind_group = device.create_bind_group(
                        "preprocess_bind_group",
                        &create_preprocess_layout(device),
                        &BindGroupEntries::sequential((
                            &tile.texture_view,
                            &tile.sampler,
                            nodes_meta_buffer.binding().unwrap(),
                        )),
                    );

                    self.preprocess_bind_group = Some(preprocess_bind_group);
                    self.processing_tasks.push(task);
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

            gpu_preprocessor.ready_tasks = preprocessor.ready_tasks.clone();
        }
    }

    pub(crate) fn prepare(
        device: Res<RenderDevice>,
        queue: Res<RenderQueue>,
        images: Res<RenderAssets<Image>>,
        gpu_node_atlases: Res<TerrainComponents<GpuNodeAtlas>>,
        mut gpu_preprocessors: ResMut<TerrainComponents<GpuPreprocessor>>,
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        for terrain in terrain_query.iter() {
            let gpu_preprocessor = gpu_preprocessors.get_mut(&terrain).unwrap();
            let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();

            gpu_preprocessor.update(&device, &queue, &images, &gpu_node_atlas);
        }
    }
}
