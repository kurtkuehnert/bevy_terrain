use crate::{
    formats::tc::save_node_config,
    terrain::Terrain,
    terrain_data::{
        node_atlas::{AtlasNode, NodeAtlas},
        NodeCoordinate,
    },
};
use bevy::{
    asset::LoadState,
    prelude::*,
    render::{render_resource::TextureFormat, texture::ImageSampler},
};
use itertools::iproduct;
use std::{collections::VecDeque, ops::DerefMut, time::Instant};

pub struct PreprocessDataset {
    pub attachment_index: usize,
    pub path: String,
}

#[derive(Clone)]
pub(crate) enum PreprocessTaskType {
    Split { tile: Handle<Image> },
    Stitch { neighbour_nodes: [AtlasNode; 8] },
    Downsample { parent_nodes: [AtlasNode; 4] },
    Barrier,
}

// Todo: store node_coordinate, task_type, node_dependencies and tile dependencies
// loop over all tasks, take n, allocate/load node and its dependencies, process task
#[derive(Clone)]
pub(crate) struct PreprocessTask {
    pub(crate) task_type: PreprocessTaskType,
    pub(crate) node: AtlasNode,
    pub(crate) attachment_index: usize,
}

fn split(
    attachment_index: usize,
    node_atlas: &mut NodeAtlas,
    tile: Handle<Image>,
    lod: u32,
    x: u32,
    y: u32,
) -> PreprocessTask {
    let node_coordinate = NodeCoordinate::new(0, lod, x, y);
    let atlas_index = node_atlas.get_or_allocate(node_coordinate);

    let node = AtlasNode {
        atlas_index,
        coordinate: node_coordinate,
    };

    PreprocessTask {
        attachment_index,
        task_type: PreprocessTaskType::Split { tile },
        node,
    }
}

fn stitch(
    attachment_index: usize,
    node_atlas: &mut NodeAtlas,
    lod: u32,
    x: u32,
    y: u32,
    node_count: u32,
) -> PreprocessTask {
    let node_coordinate = NodeCoordinate::new(0, lod, x, y);
    let atlas_index = node_atlas.get_or_allocate(node_coordinate);

    let node = AtlasNode {
        atlas_index,
        coordinate: node_coordinate,
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

    let mut neighbour_nodes = [AtlasNode::default(); 8];

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
            || neighbour_node_position.x >= node_count as i32
            || neighbour_node_position.y >= node_count as i32
        {
            u32::MAX
        } else {
            node_atlas.get_or_allocate(neighbour_node_coordinate)
        };

        neighbour_nodes[index] = AtlasNode {
            coordinate: neighbour_node_coordinate,
            atlas_index: neighbour_atlas_index,
        };
    }

    PreprocessTask {
        attachment_index,
        task_type: PreprocessTaskType::Stitch { neighbour_nodes },
        node,
    }
}

fn downsample(
    attachment_index: usize,
    node_atlas: &mut NodeAtlas,
    lod: u32,
    x: u32,
    y: u32,
) -> PreprocessTask {
    let node_coordinate = NodeCoordinate::new(0, lod, x, y);
    let atlas_index = node_atlas.get_or_allocate(node_coordinate);

    let node = AtlasNode {
        atlas_index,
        coordinate: node_coordinate,
    };

    let mut parent_nodes = [AtlasNode::default(); 4];

    for index in 0..4 {
        let parent_node_coordinate =
            NodeCoordinate::new(0, lod - 1, 2 * x + index % 2, 2 * y + index / 2);
        let parent_atlas_index = node_atlas.get_or_allocate(parent_node_coordinate);

        parent_nodes[index as usize] = AtlasNode {
            coordinate: parent_node_coordinate,
            atlas_index: parent_atlas_index,
        };
    }

    PreprocessTask {
        attachment_index,
        task_type: PreprocessTaskType::Downsample { parent_nodes },
        node,
    }
}

#[derive(Component)]
pub struct Preprocessor {
    pub(crate) path: String,
    pub(crate) loading_tiles: Vec<Handle<Image>>,
    pub(crate) task_queue: VecDeque<PreprocessTask>,
    pub(crate) ready_tasks: Vec<PreprocessTask>,

    pub(crate) start_time: Option<Instant>,
}

impl Preprocessor {
    pub fn new(path: String) -> Self {
        Self {
            path,
            loading_tiles: default(),
            task_queue: default(),
            ready_tasks: default(),
            start_time: default(),
        }
    }

    pub fn preprocess_tile(
        &mut self,
        dataset: PreprocessDataset,
        asset_server: &AssetServer,
        node_atlas: &mut NodeAtlas,
    ) {
        let tile_handle = asset_server.load(dataset.path);
        self.loading_tiles.push(tile_handle.clone());

        let lod_count = node_atlas.lod_count;
        let node_count = 1 << (lod_count - 1);
        let lod = 0;

        for (x, y) in iproduct!(0..node_count, 0..node_count) {
            self.task_queue.push_back(split(
                dataset.attachment_index,
                node_atlas,
                tile_handle.clone(),
                lod,
                x,
                y,
            ));
        }

        self.task_queue.push_back(PreprocessTask {
            attachment_index: dataset.attachment_index,
            task_type: PreprocessTaskType::Barrier,
            node: Default::default(),
        });

        for (x, y) in iproduct!(0..node_count, 0..node_count) {
            self.task_queue.push_back(stitch(
                dataset.attachment_index,
                node_atlas,
                lod,
                x,
                y,
                node_count,
            ));
        }

        for lod in 1..lod_count {
            let node_count = node_count >> lod;

            for (x, y) in iproduct!(0..node_count, 0..node_count) {
                self.task_queue.push_back(downsample(
                    dataset.attachment_index,
                    node_atlas,
                    lod,
                    x,
                    y,
                ));
            }

            self.task_queue.push_back(PreprocessTask {
                attachment_index: dataset.attachment_index,
                task_type: PreprocessTaskType::Barrier,
                node: Default::default(),
            });

            for (x, y) in iproduct!(0..node_count, 0..node_count) {
                self.task_queue.push_back(stitch(
                    dataset.attachment_index,
                    node_atlas,
                    lod,
                    x,
                    y,
                    node_count,
                ));
            }
        }

        self.start_time = Some(Instant::now());
    }
}

pub(crate) fn select_ready_tasks(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<(&mut Preprocessor, &mut NodeAtlas), With<Terrain>>,
) {
    for (mut preprocessor, mut node_atlas) in terrain_query.iter_mut() {
        let Preprocessor {
            path,
            task_queue,
            ready_tasks,
            start_time,
            ..
        } = preprocessor.deref_mut();

        if task_queue.is_empty() && node_atlas.state.slots == node_atlas.state.max_slots {
            if let Some(start) = start_time {
                let elapsed = start.elapsed();
                *start_time = None;

                dbg!(elapsed);

                save_node_config(path);
            }
        }

        ready_tasks.clear();

        while node_atlas.state.slots > 0 {
            let ready = task_queue
                .front()
                .map_or(false, |task| match &task.task_type {
                    PreprocessTaskType::Split { tile } => {
                        asset_server.load_state(tile) == LoadState::Loaded
                    }
                    PreprocessTaskType::Stitch { .. } => true,
                    PreprocessTaskType::Downsample { .. } => true,
                    PreprocessTaskType::Barrier => {
                        if node_atlas.state.slots == node_atlas.state.max_slots {
                            dbg!("barrier complete");
                        }
                        node_atlas.state.slots == node_atlas.state.max_slots
                    }
                });

            if ready {
                let task = task_queue.pop_front().unwrap();

                if !matches!(task.task_type, PreprocessTaskType::Barrier) {
                    ready_tasks.push(task);
                    node_atlas.state.slots -= 1;
                }
            } else {
                break;
            }
        }
    }
}

pub(crate) fn preprocessor_load_tile(
    mut terrain_query: Query<&mut Preprocessor, With<Terrain>>,
    mut images: ResMut<Assets<Image>>,
) {
    for mut preprocessor in terrain_query.iter_mut() {
        preprocessor.loading_tiles.retain_mut(|handle| {
            if let Some(image) = images.get_mut(handle.id()) {
                image.texture_descriptor.format = TextureFormat::R16Unorm;
                image.sampler = ImageSampler::linear();

                false
            } else {
                true
            }
        });
    }
}
