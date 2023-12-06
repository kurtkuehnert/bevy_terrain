use crate::{
    prelude::{NodeAtlas, TileConfig},
    preprocess_gpu::gpu_preprocessor::NodeMeta,
    terrain::Terrain,
    terrain_data::NodeCoordinate,
};
use bevy::{
    asset::LoadState,
    prelude::*,
    render::{render_resource::TextureFormat, texture::ImageSampler},
};
use itertools::iproduct;
use std::collections::VecDeque;
use std::mem;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) enum PreprocessTaskType {
    SplitTile { tile: Handle<Image> },
    Stitch,
    Downsample,
}

// Todo: store node_coordinate, task_type, node_dependencies and tile dependencies
// loop over all tasks, take n, allocate/load node and its dependencies, process task
#[derive(Clone)]
pub(crate) struct PreprocessTask {
    pub(crate) task_type: PreprocessTaskType,
    pub(crate) node: NodeMeta,
}

#[derive(Component)]
pub struct Preprocessor {
    pub(crate) tile_handle: Option<Handle<Image>>,
    pub(crate) task_queue: VecDeque<PreprocessTask>,
    pub(crate) ready_tasks: Vec<PreprocessTask>,
    pub(crate) finished_tasks: Arc<Mutex<Vec<PreprocessTask>>>,
    pub(crate) blocked: bool,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            tile_handle: None,
            task_queue: default(),
            ready_tasks: vec![],
            finished_tasks: Arc::new(Mutex::new(vec![])),
            blocked: false,
        }
    }

    pub fn preprocess_tile(
        &mut self,
        tile_config: TileConfig,
        asset_server: &AssetServer,
        node_atlas: &mut NodeAtlas,
    ) {
        self.tile_handle = Some(asset_server.load(tile_config.path));

        for (x, y) in iproduct!(0..4, 0..4) {
            let node_coordinate = NodeCoordinate::new(0, 0, x, y);

            let atlas_index = node_atlas.allocate(node_coordinate.clone());

            let node = NodeMeta {
                atlas_index: atlas_index as u32,
                _padding: 0,
                node_coordinate,
            };

            self.task_queue.push_back(PreprocessTask {
                task_type: PreprocessTaskType::SplitTile {
                    tile: self.tile_handle.clone().unwrap(),
                },
                node,
            });
        }

        // for (x, y) in iproduct!(0..2, 0..2) {
        //     let node_coordinate = NodeCoordinate::new(0, 0, x, y);
        //     let atlas_index = 0; // Todo: get from node_atlas
        //
        //     let node = NodeMeta {
        //         atlas_index: atlas_index as u32,
        //         _padding: 0,
        //         node_coordinate,
        //     };
        //
        //     self.task_queue.push_back(PreprocessTask {
        //         task_type: PreprocessTaskType::Stitch,
        //         node,
        //     });
        // }
    }
}

pub(crate) fn select_ready_tasks(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<&mut Preprocessor, With<Terrain>>,
) {
    for mut preprocessor in terrain_query.iter_mut() {
        preprocessor.ready_tasks = vec![];

        let finished_tasks = mem::take(preprocessor.finished_tasks.lock().unwrap().deref_mut());

        if !finished_tasks.is_empty() {
            preprocessor.blocked = false;
        }

        // Todo: loop multiple times
        let ready = if let Some(task) = preprocessor.task_queue.front() {
            match &task.task_type {
                PreprocessTaskType::SplitTile { tile } => {
                    asset_server.load_state(tile) == LoadState::Loaded
                }
                PreprocessTaskType::Stitch => false,
                PreprocessTaskType::Downsample => false,
            }
        } else {
            false
        };

        if ready && !preprocessor.blocked {
            let task = preprocessor.task_queue.pop_front().unwrap();

            preprocessor.ready_tasks.push(task);
            preprocessor.blocked = true;
        }
    }
}

pub(crate) fn preprocessor_load_tile(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<&mut Preprocessor, With<Terrain>>,
    mut images: ResMut<Assets<Image>>,
) {
    for mut preprocessor in terrain_query.iter_mut() {
        if let Some(handle) = &preprocessor.tile_handle {
            if asset_server.load_state(handle) == LoadState::Loaded {
                let image = images.get_mut(handle).unwrap();
                image.texture_descriptor.format = TextureFormat::R16Unorm;
                image.sampler = ImageSampler::linear();

                preprocessor.tile_handle = None;
            }
        }
    }
}
