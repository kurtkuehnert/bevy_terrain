use crate::{
    formats::tc::save_node_config,
    terrain::Terrain,
    terrain_data::{
        coordinates::NodeCoordinate,
        node_atlas::{AtlasNode, AtlasNodeAttachment, NodeAtlas},
        AttachmentFormat,
    },
};
use bevy::{asset::LoadState, prelude::*, render::texture::ImageSampler};
use itertools::{iproduct, Itertools};
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
    Downsample { child_nodes: [AtlasNode; 4] },
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

    fn stitch(node: AtlasNodeAttachment, node_atlas: &mut NodeAtlas) -> Self {
        let neighbour_nodes = node
            .coordinate
            .neighbours(node_atlas.lod_count)
            .iter()
            .map(|&coordinate| node_atlas.get_or_allocate(coordinate))
            .collect_vec()
            .try_into()
            .unwrap();

        Self {
            node,
            task_type: PreprocessTaskType::Stitch { neighbour_nodes },
        }
    }

    fn downsample(node: AtlasNodeAttachment, node_atlas: &mut NodeAtlas) -> Self {
        let child_nodes = node
            .coordinate
            .children()
            .iter()
            .map(|&coordinate| node_atlas.get_or_allocate(coordinate))
            .collect_vec()
            .try_into()
            .unwrap();

        Self {
            node,
            task_type: PreprocessTaskType::Downsample { child_nodes },
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
    loaded: bool,
}

impl Preprocessor {
    pub fn new(path: String) -> Self {
        Self {
            path,
            loading_tiles: default(),
            task_queue: default(),
            ready_tasks: default(),
            start_time: None,
            loaded: false,
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
        let lod = 0;
        let node_count = 1 << (lod_count - lod - 1);

        for (x, y) in iproduct!(0..node_count, 0..node_count) {
            let node = node_atlas
                .get_or_allocate(NodeCoordinate::new(dataset.side, lod, x, y))
                .attachment(dataset.attachment_index);

            self.task_queue
                .push_back(PreprocessTask::split(node, tile_handle.clone()));
        }

        for lod in 1..lod_count {
            let node_count = 1 << (lod_count - lod - 1);

            self.task_queue.push_back(PreprocessTask::barrier());

            for (x, y) in iproduct!(0..node_count, 0..node_count) {
                let node = node_atlas
                    .get_or_allocate(NodeCoordinate::new(dataset.side, lod, x, y))
                    .attachment(dataset.attachment_index);

                self.task_queue
                    .push_back(PreprocessTask::downsample(node, node_atlas));
            }
        }

        self.task_queue.push_back(PreprocessTask::barrier());

        for lod in 0..lod_count {
            let node_count = 1 << (lod_count - lod - 1);

            for (x, y) in iproduct!(0..node_count, 0..node_count) {
                let node = node_atlas
                    .get_or_allocate(NodeCoordinate::new(dataset.side, lod, x, y))
                    .attachment(dataset.attachment_index);

                self.task_queue
                    .push_back(PreprocessTask::stitch(node, node_atlas));
            }

            self.task_queue.push_back(PreprocessTask::barrier());

            for (x, y) in iproduct!(0..node_count, 0..node_count) {
                let node = node_atlas
                    .get_or_allocate(NodeCoordinate::new(dataset.side, lod, x, y))
                    .attachment(dataset.attachment_index);

                self.task_queue.push_back(PreprocessTask::save(node));
            }
        }
    }

    pub fn preprocess_spherical(
        &mut self,
        dataset: PreprocessDataset,
        asset_server: &AssetServer,
        node_atlas: &mut NodeAtlas,
    ) {
        for side in 0..6 {
            self.preprocess_tile(
                PreprocessDataset {
                    attachment_index: dataset.attachment_index,
                    path: format!("{}/source/height/face{}.png", dataset.path, side),
                    side,
                },
                asset_server,
                node_atlas,
            );
        }

        let lod_count = node_atlas.lod_count;

        self.task_queue.push_back(PreprocessTask::barrier());

        for side in 0..6 {
            for lod in 0..lod_count {
                let node_count = 1 << (lod_count - lod - 1);

                for (x, y) in iproduct!(0..node_count, 0..node_count) {
                    let node = node_atlas
                        .get_or_allocate(NodeCoordinate::new(side, lod, x, y))
                        .attachment(dataset.attachment_index);

                    self.task_queue
                        .push_back(PreprocessTask::stitch(node, node_atlas));
                }

                self.task_queue.push_back(PreprocessTask::barrier());

                for (x, y) in iproduct!(0..node_count, 0..node_count) {
                    let node = node_atlas
                        .get_or_allocate(NodeCoordinate::new(side, lod, x, y))
                        .attachment(dataset.attachment_index);

                    self.task_queue.push_back(PreprocessTask::save(node));
                }
            }
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

        if let Some(time) = start_time {
            if task_queue.is_empty()
                && node_atlas.state.download_slots == node_atlas.state.max_download_slots
                && node_atlas.state.save_slots == node_atlas.state.max_save_slots
            {
                println!("Preprocessing took {:?}", time.elapsed());

                save_node_config(path);

                *start_time = None;
            }
        } else {
            break;
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
                    // dbg!("barrier complete");
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

        if !preprocessor.loaded && preprocessor.loading_tiles.is_empty() {
            println!("finished_loading all tiles");
            preprocessor.loaded = true;
            preprocessor.start_time = Some(Instant::now());
        }
    }
}
