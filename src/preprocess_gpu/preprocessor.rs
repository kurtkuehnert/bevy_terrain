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

#[derive(Clone)]
pub(crate) enum PreprocessTaskType {
    SplitTile { tile: Handle<Image> },
    Stitch { neighbour_nodes: [NodeMeta; 8] },
    Downsample { parent_nodes: [NodeMeta; 4] },
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

fn split(
    node_atlas: &mut NodeAtlas,
    tile: Handle<Image>,
    lod: u32,
    x: u32,
    y: u32,
) -> PreprocessTask {
    let node_coordinate = NodeCoordinate::new(0, lod, x, y);
    let atlas_index = node_atlas.get_or_allocate(node_coordinate);

    let node = NodeMeta {
        atlas_index: atlas_index as u32,
        node_coordinate,
    };

    PreprocessTask {
        task_type: PreprocessTaskType::SplitTile { tile },
        node,
    }
}

fn stitch(
    node_atlas: &mut NodeAtlas,
    lod: u32,
    x: u32,
    y: u32,
    node_width: u32,
    node_height: u32,
) -> PreprocessTask {
    let node_coordinate = NodeCoordinate::new(0, lod, x, y);
    let atlas_index = node_atlas.get_or_allocate(node_coordinate) as u32;

    let node = NodeMeta {
        atlas_index,
        node_coordinate,
    };

    let offsets = [
        IVec2::new(0, -1),
        IVec2::new(1, 0),
        IVec2::new(0, 1),
        IVec2::new(-1, 0),
        IVec2::new(-1, -1),
        IVec2::new(1, -1),
        IVec2::new(1, 1),
        IVec2::new(-1, 1),
    ];

    let node_position = IVec2::new(x as i32, y as i32);

    let mut neighbour_nodes = [NodeMeta::default(); 8];

    for (index, &offset) in offsets.iter().enumerate() {
        let neighbour_node_position = node_position + offset;

        let neighbour_node_coordinate = NodeCoordinate::new(
            node_coordinate.side,
            node_coordinate.lod,
            neighbour_node_position.x as u32,
            neighbour_node_position.y as u32,
        );

        let neighbour_atlas_index = if neighbour_node_position.x < 0
            || neighbour_node_position.y < 0
            || neighbour_node_position.x >= node_width as i32
            || neighbour_node_position.y >= node_height as i32
        {
            u32::MAX
        } else {
            node_atlas.get_or_allocate(neighbour_node_coordinate) as u32
        };

        neighbour_nodes[index] = NodeMeta {
            node_coordinate: neighbour_node_coordinate,
            atlas_index: neighbour_atlas_index,
        };
    }

    PreprocessTask {
        task_type: PreprocessTaskType::Stitch { neighbour_nodes },
        node,
    }
}

fn downsample(node_atlas: &mut NodeAtlas, lod: u32, x: u32, y: u32) -> PreprocessTask {
    let node_coordinate = NodeCoordinate::new(0, lod, x, y);
    let atlas_index = node_atlas.get_or_allocate(node_coordinate);

    let node = NodeMeta {
        atlas_index: atlas_index as u32,
        node_coordinate,
    };

    let mut parent_nodes = [NodeMeta::default(); 4];

    for index in 0..4 {
        let parent_node_coordinate =
            NodeCoordinate::new(0, lod - 1, 2 * x + index % 2, 2 * y + index / 2);
        let parent_atlas_index = node_atlas.get_or_allocate(parent_node_coordinate) as u32;

        parent_nodes[index as usize] = NodeMeta {
            node_coordinate: parent_node_coordinate,
            atlas_index: parent_atlas_index,
        };
    }

    PreprocessTask {
        task_type: PreprocessTaskType::Downsample { parent_nodes },
        node,
    }
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            tile_handle: None,
            task_queue: default(),
            ready_tasks: default(),
            read_back_tasks: default(),
            saving_tasks: default(),
            slots: 1,
        }
    }

    pub fn preprocess_tile(
        &mut self,
        tile_config: TileConfig,
        asset_server: &AssetServer,
        node_atlas: &mut NodeAtlas,
    ) {
        let tile_handle = asset_server.load(tile_config.path);
        self.tile_handle = Some(tile_handle.clone());

        let node_width = 4;
        let node_height = 4;
        let lod_count = 3;
        let lod = 0;

        for (x, y) in iproduct!(0..node_width, 0..node_height) {
            self.task_queue
                .push_back(split(node_atlas, tile_handle.clone(), lod, x, y));
        }

        for (x, y) in iproduct!(0..node_width, 0..node_height) {
            self.task_queue
                .push_back(stitch(node_atlas, lod, x, y, node_width, node_height));
        }

        for lod in 1..lod_count {
            let node_width = node_width >> lod;
            let node_height = node_height >> lod;

            for (x, y) in iproduct!(0..node_width, 0..node_height) {
                self.task_queue.push_back(downsample(node_atlas, lod, x, y));
            }

            for (x, y) in iproduct!(0..node_width, 0..node_height) {
                self.task_queue
                    .push_back(stitch(node_atlas, lod, x, y, node_width, node_height));
            }
        }
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
                    PreprocessTaskType::Stitch { .. } => true,
                    PreprocessTaskType::Downsample { .. } => true,
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
