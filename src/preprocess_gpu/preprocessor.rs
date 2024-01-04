use crate::terrain_data::node_atlas::AtlasNodeAttachment;
use crate::terrain_data::INVALID_ATLAS_INDEX;
use crate::{
    formats::tc::save_node_config,
    terrain::Terrain,
    terrain_data::{
        node_atlas::{AtlasNode, NodeAtlas},
        AttachmentFormat, NodeCoordinate,
    },
};
use bevy::{asset::LoadState, prelude::*, render::texture::ImageSampler};
use itertools::iproduct;
use std::{collections::VecDeque, ops::DerefMut, time::Instant};

pub(crate) struct LoadingTile {
    id: AssetId<Image>,
    format: AttachmentFormat,
}

pub struct PreprocessDataset {
    pub attachment_index: u32,
    pub path: String,
    pub side: u32,
}

#[derive(Clone)]
pub(crate) enum PreprocessTaskType {
    Split { tile: Handle<Image> },
    Stitch { neighbour_nodes: [AtlasNode; 8] },
    Downsample { parent_nodes: [AtlasNode; 4] },
    Save,
    Barrier,
}

// Todo: store node_coordinate, task_type, node_dependencies and tile dependencies
// loop over all tasks, take n, allocate/load node and its dependencies, process task
#[derive(Clone)]
pub(crate) struct PreprocessTask {
    pub(crate) node: AtlasNodeAttachment,
    pub(crate) task_type: PreprocessTaskType,
}

impl PreprocessTask {
    fn is_ready(&self, asset_server: &AssetServer, node_atlas: &NodeAtlas) -> bool {
        match &self.task_type {
            PreprocessTaskType::Split { tile } => {
                asset_server.load_state(tile) == LoadState::Loaded
            }
            PreprocessTaskType::Stitch { .. } => true,
            PreprocessTaskType::Downsample { .. } => true,
            PreprocessTaskType::Barrier => {
                node_atlas.state.download_slots == node_atlas.state.max_download_slots
            }
            PreprocessTaskType::Save => true,
        }
    }

    fn barrier() -> Self {
        Self {
            node: default(),
            task_type: PreprocessTaskType::Barrier,
        }
    }

    fn save(node: AtlasNodeAttachment) -> Self {
        Self {
            node,
            task_type: PreprocessTaskType::Save,
        }
    }

    fn split(node: AtlasNodeAttachment, tile: Handle<Image>) -> Self {
        Self {
            node,
            task_type: PreprocessTaskType::Split { tile },
        }
    }

    fn stitch(node: AtlasNodeAttachment, node_atlas: &mut NodeAtlas, node_count: u32) -> Self {
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

        let node_position = IVec2::new(node.coordinate.x as i32, node.coordinate.y as i32);

        let mut neighbour_nodes = [AtlasNode::default(); 8];

        for (index, &offset) in offsets.iter().enumerate() {
            let neighbour_node_position = node_position + offset;

            let neighbour_node_coordinate = NodeCoordinate::new(
                node.coordinate.side,
                node.coordinate.lod,
                neighbour_node_position.x as u32,
                neighbour_node_position.y as u32,
            );

            neighbour_nodes[index] = if neighbour_node_position.x < 0
                || neighbour_node_position.y < 0
                || neighbour_node_position.x >= node_count as i32
                || neighbour_node_position.y >= node_count as i32
            {
                AtlasNode::new(neighbour_node_coordinate, INVALID_ATLAS_INDEX)
            } else {
                node_atlas.get_or_allocate(neighbour_node_coordinate)
            };
        }

        Self {
            node,
            task_type: PreprocessTaskType::Stitch { neighbour_nodes },
        }
    }

    fn downsample(node: AtlasNodeAttachment, node_atlas: &mut NodeAtlas) -> Self {
        let mut parent_nodes = [AtlasNode::default(); 4];

        for index in 0..4 {
            let parent_node_coordinate = NodeCoordinate::new(
                node.coordinate.side,
                node.coordinate.lod - 1,
                2 * node.coordinate.x + index % 2,
                2 * node.coordinate.y + index / 2,
            );

            parent_nodes[index as usize] = node_atlas.get_or_allocate(parent_node_coordinate);
        }

        Self {
            node,
            task_type: PreprocessTaskType::Downsample { parent_nodes },
        }
    }
}

#[derive(Component)]
pub struct Preprocessor {
    pub(crate) path: String,
    pub(crate) loading_tiles: Vec<LoadingTile>,
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
            start_time: Some(Instant::now()),
        }
    }

    pub fn preprocess_tile(
        &mut self,
        dataset: PreprocessDataset,
        asset_server: &AssetServer,
        node_atlas: &mut NodeAtlas,
    ) {
        let tile_handle = asset_server.load(dataset.path);

        self.loading_tiles.push(LoadingTile {
            id: tile_handle.id(),
            format: node_atlas.attachments[dataset.attachment_index as usize].format,
        });

        let lod_count = node_atlas.lod_count;
        let node_count = 1 << (lod_count - 1);
        let lod = 0;

        let mut nodes = Vec::new();

        for (x, y) in iproduct!(0..node_count, 0..node_count) {
            let node = node_atlas
                .get_or_allocate(NodeCoordinate::new(dataset.side, lod, x, y))
                .attachment(dataset.attachment_index);

            self.task_queue
                .push_back(PreprocessTask::split(node, tile_handle.clone()));
        }

        self.task_queue.push_back(PreprocessTask::barrier());

        for (x, y) in iproduct!(0..node_count, 0..node_count) {
            let node = node_atlas
                .get_or_allocate(NodeCoordinate::new(dataset.side, lod, x, y))
                .attachment(dataset.attachment_index);

            self.task_queue
                .push_back(PreprocessTask::stitch(node, node_atlas, node_count));

            nodes.push(node);
        }

        for lod in 1..lod_count {
            let node_count = node_count >> lod;

            for (x, y) in iproduct!(0..node_count, 0..node_count) {
                let node = node_atlas
                    .get_or_allocate(NodeCoordinate::new(dataset.side, lod, x, y))
                    .attachment(dataset.attachment_index);

                self.task_queue
                    .push_back(PreprocessTask::downsample(node, node_atlas));
            }

            self.task_queue.push_back(PreprocessTask::barrier());

            for (x, y) in iproduct!(0..node_count, 0..node_count) {
                let node = node_atlas
                    .get_or_allocate(NodeCoordinate::new(dataset.side, lod, x, y))
                    .attachment(dataset.attachment_index);

                self.task_queue
                    .push_back(PreprocessTask::stitch(node, node_atlas, node_count));

                nodes.push(node);
            }
        }

        self.task_queue.push_back(PreprocessTask::barrier());

        for node in nodes {
            self.task_queue.push_back(PreprocessTask::save(node));
        }
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

        if task_queue.is_empty()
            && node_atlas.state.download_slots == node_atlas.state.max_download_slots
        {
            if let Some(start) = start_time {
                let elapsed = start.elapsed();
                *start_time = None;

                dbg!(elapsed);

                save_node_config(path);
            }
        }

        ready_tasks.clear();

        loop {
            if (node_atlas.state.download_slots > 0)
                && task_queue
                    .front()
                    .map_or(false, |task| task.is_ready(&asset_server, &node_atlas))
            {
                let task = task_queue.pop_front().unwrap();

                if matches!(task.task_type, PreprocessTaskType::Save) {
                    node_atlas.save(task.node);
                } else if matches!(task.task_type, PreprocessTaskType::Barrier) {
                    dbg!("barrier complete");
                } else {
                    ready_tasks.push(task);
                    node_atlas.state.download_slots -= 1;
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
        preprocessor.loading_tiles.retain_mut(|tile| {
            if let Some(image) = images.get_mut(tile.id) {
                image.texture_descriptor.format = tile.format.processing_format();
                image.sampler = ImageSampler::linear();

                false
            } else {
                true
            }
        });
    }
}
