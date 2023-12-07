use crate::{
    prelude::{NodeAtlas, TileConfig},
    preprocess_gpu::gpu_preprocessor::{NodeMeta, ReadBackNode},
    terrain::Terrain,
    terrain_data::NodeCoordinate,
};
use bevy::{
    asset::LoadState,
    prelude::*,
    render::{render_resource::TextureFormat, texture::ImageSampler},
    tasks::{futures_lite::future, Task},
};
use itertools::iproduct;
use std::{
    collections::VecDeque,
    ops::DerefMut,
    sync::{Arc, Mutex},
};

#[derive(Clone, Eq, PartialEq)]
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
    pub(crate) read_back_tasks: Arc<Mutex<Vec<Task<Vec<ReadBackNode>>>>>,
    pub(crate) saving_tasks: Vec<Task<()>>,
    pub(crate) slots: u32,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            tile_handle: None,
            task_queue: default(),
            ready_tasks: default(),
            read_back_tasks: default(),
            saving_tasks: default(),
            slots: 8,
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
        let Preprocessor {
            task_queue,
            ready_tasks,
            read_back_tasks,
            saving_tasks,
            slots,
            ..
        } = preprocessor.deref_mut();

        saving_tasks.retain_mut(|task| {
            if future::block_on(future::poll_once(task)).is_some() {
                *slots += 1;
                false
            } else {
                true
            }
        });

        read_back_tasks
            .lock()
            .unwrap()
            .deref_mut()
            .retain_mut(|task| {
                if let Some(nodes) = future::block_on(future::poll_once(task)) {
                    for node in nodes {
                        saving_tasks.push(node.start_saving());
                    }
                    false
                } else {
                    true
                }
            });

        ready_tasks.clear();

        while *slots > 0 {
            let ready = task_queue
                .front()
                .map_or(false, |task| match &task.task_type {
                    PreprocessTaskType::SplitTile { tile } => {
                        asset_server.load_state(tile) == LoadState::Loaded
                    }
                    PreprocessTaskType::Stitch => false,
                    PreprocessTaskType::Downsample => false,
                });

            if ready {
                let task = task_queue.pop_front().unwrap();

                ready_tasks.push(task);
                *slots -= 1;
            } else {
                break;
            }
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
