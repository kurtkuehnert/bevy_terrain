use crate::{
    math::NodeCoordinate,
    terrain::Terrain,
    terrain_data::{
        node_atlas::{AtlasNode, AtlasNodeAttachment, NodeAtlas},
        AttachmentFormat,
    },
    util::CollectArray,
};
use bevy::{prelude::*, render::texture::ImageSampler};
use itertools::{iproduct, Itertools};
use std::{
    collections::VecDeque,
    fs,
    ops::{DerefMut, Range},
    time::Instant,
};

pub fn reset_directory(directory: &str) {
    let _ = fs::remove_file(format!("{directory}/../../config.tc"));
    let _ = fs::remove_dir_all(directory);
    fs::create_dir_all(directory).unwrap();
}

pub(crate) struct LoadingTile {
    id: AssetId<Image>,
    format: AttachmentFormat,
}

pub struct SphericalDataset {
    pub attachment_index: u32,
    pub paths: Vec<String>,
    pub lod_range: Range<u32>,
}

pub struct PreprocessDataset {
    pub attachment_index: u32,
    pub path: String,
    pub side: u32,
    pub top_left: Vec2,
    pub bottom_right: Vec2,
    pub lod_range: Range<u32>,
}

impl Default for PreprocessDataset {
    fn default() -> Self {
        Self {
            attachment_index: 0,
            path: "".to_string(),
            side: 0,
            top_left: Vec2::splat(0.0),
            bottom_right: Vec2::splat(1.0),
            lod_range: 0..1,
        }
    }
}

impl PreprocessDataset {
    fn overlapping_nodes(&self, lod: u32) -> impl Iterator<Item = NodeCoordinate> + '_ {
        let node_count = NodeCoordinate::node_count(lod);

        let lower = (self.top_left * node_count as f32).as_uvec2();
        let upper = (self.bottom_right * node_count as f32).ceil().as_uvec2();

        iproduct!(lower.x..upper.x, lower.y..upper.y)
            .map(move |(x, y)| NodeCoordinate::new(self.side, lod, x, y))
    }
}

#[derive(Clone)]
pub(crate) enum PreprocessTaskType {
    Split {
        tile: Handle<Image>,
        top_left: Vec2,
        bottom_right: Vec2,
    },
    Stitch {
        neighbour_nodes: [AtlasNode; 8],
    },
    Downsample {
        child_nodes: [AtlasNode; 4],
    },
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
            PreprocessTaskType::Split { tile, .. } => {
                asset_server.is_loaded_with_dependencies(tile)
            }
            PreprocessTaskType::Stitch { .. } => true,
            PreprocessTaskType::Downsample { .. } => true,
            PreprocessTaskType::Barrier => {
                node_atlas.state.download_slots == node_atlas.state.max_download_slots
            }
            PreprocessTaskType::Save => true,
        }
    }

    #[allow(dead_code)]
    fn debug(&self) {
        match &self.task_type {
            PreprocessTaskType::Split { .. } => {
                println!("Splitting node: {}", self.node.coordinate)
            }
            PreprocessTaskType::Stitch { .. } => {
                println!("Stitching node: {}", self.node.coordinate)
            }
            PreprocessTaskType::Downsample { .. } => {
                println!("Downsampling node: {}", self.node.coordinate)
            }
            PreprocessTaskType::Save => println!("Started saving node: {}", self.node.coordinate),
            PreprocessTaskType::Barrier => println!("Barrier"),
        }
    }

    fn barrier() -> Self {
        Self {
            node: default(),
            task_type: PreprocessTaskType::Barrier,
        }
    }

    fn save(
        node_coordinate: NodeCoordinate,
        node_atlas: &mut NodeAtlas,
        dataset: &PreprocessDataset,
    ) -> Self {
        let node = node_atlas
            .get_or_allocate_node(node_coordinate)
            .attachment(dataset.attachment_index);

        Self {
            node,
            task_type: PreprocessTaskType::Save,
        }
    }

    fn split(
        node_coordinate: NodeCoordinate,
        node_atlas: &mut NodeAtlas,
        dataset: &PreprocessDataset,
        tile: Handle<Image>,
    ) -> Self {
        let node = node_atlas
            .get_or_allocate_node(node_coordinate)
            .attachment(dataset.attachment_index);

        Self {
            node,
            task_type: PreprocessTaskType::Split {
                tile,
                top_left: dataset.top_left,
                bottom_right: dataset.bottom_right,
            },
        }
    }

    fn stitch(
        node_coordinate: NodeCoordinate,
        node_atlas: &mut NodeAtlas,
        dataset: &PreprocessDataset,
    ) -> Self {
        let node = node_atlas
            .get_or_allocate_node(node_coordinate)
            .attachment(dataset.attachment_index);

        let neighbour_nodes = node
            .coordinate
            .neighbours(node_atlas.spherical)
            .map(|coordinate| node_atlas.get_node(coordinate))
            .collect_array();

        Self {
            node,
            task_type: PreprocessTaskType::Stitch { neighbour_nodes },
        }
    }

    fn downsample(
        node_coordinate: NodeCoordinate,
        node_atlas: &mut NodeAtlas,
        dataset: &PreprocessDataset,
    ) -> Self {
        let node = node_atlas
            .get_or_allocate_node(node_coordinate)
            .attachment(dataset.attachment_index);

        let child_nodes = node
            .coordinate
            .children()
            .map(|coordinate| node_atlas.get_node(coordinate))
            .collect_array();

        Self {
            node,
            task_type: PreprocessTaskType::Downsample { child_nodes },
        }
    }
}

#[derive(Component)]
pub struct Preprocessor {
    pub(crate) loading_tiles: Vec<LoadingTile>,
    pub(crate) task_queue: VecDeque<PreprocessTask>,
    pub(crate) ready_tasks: Vec<PreprocessTask>,

    pub(crate) start_time: Option<Instant>,
    loaded: bool,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            loading_tiles: default(),
            task_queue: default(),
            ready_tasks: default(),
            start_time: None,
            loaded: false,
        }
    }

    fn split_and_downsample(
        &mut self,
        dataset: &PreprocessDataset,
        asset_server: &AssetServer,
        node_atlas: &mut NodeAtlas,
    ) {
        let tile_handle = asset_server.load(&dataset.path);

        self.loading_tiles.push(LoadingTile {
            id: tile_handle.id(),
            format: node_atlas.attachments[dataset.attachment_index as usize].format,
        });

        let mut lods = dataset.lod_range.clone().rev();

        for node_coordinate in dataset.overlapping_nodes(lods.next().unwrap()) {
            self.task_queue.push_back(PreprocessTask::split(
                node_coordinate,
                node_atlas,
                dataset,
                tile_handle.clone(),
            ));
        }

        for lod in lods {
            self.task_queue.push_back(PreprocessTask::barrier());

            for node_coordinate in dataset.overlapping_nodes(lod) {
                self.task_queue.push_back(PreprocessTask::downsample(
                    node_coordinate,
                    node_atlas,
                    dataset,
                ));
            }
        }
    }

    fn stitch_and_save_layer(
        &mut self,
        dataset: &PreprocessDataset,
        node_atlas: &mut NodeAtlas,
        lod: u32,
    ) {
        for node_coordinate in dataset.overlapping_nodes(lod) {
            self.task_queue
                .push_back(PreprocessTask::stitch(node_coordinate, node_atlas, dataset));
        }

        self.task_queue.push_back(PreprocessTask::barrier());

        for node_coordinate in dataset.overlapping_nodes(lod) {
            self.task_queue
                .push_back(PreprocessTask::save(node_coordinate, node_atlas, dataset));
        }
    }

    pub fn clear_attachment(self, attachment_index: u32, node_atlas: &mut NodeAtlas) -> Self {
        let attachment = &mut node_atlas.attachments[attachment_index as usize];
        node_atlas.state.existing_nodes.clear();
        reset_directory(&attachment.path);

        self
    }

    pub fn preprocess_tile(
        mut self,
        dataset: PreprocessDataset,
        asset_server: &AssetServer,
        node_atlas: &mut NodeAtlas,
    ) -> Self {
        self.split_and_downsample(&dataset, asset_server, node_atlas);
        self.task_queue.push_back(PreprocessTask::barrier());

        for lod in dataset.lod_range.clone() {
            self.stitch_and_save_layer(&dataset, node_atlas, lod);
        }

        self
    }

    pub fn preprocess_spherical(
        mut self,
        dataset: SphericalDataset,
        asset_server: &AssetServer,
        node_atlas: &mut NodeAtlas,
    ) -> Self {
        let side_datasets = (0..6)
            .map(|side| PreprocessDataset {
                attachment_index: dataset.attachment_index,
                path: dataset.paths[side as usize].clone(),
                side,
                lod_range: dataset.lod_range.clone(),
                ..default()
            })
            .collect_vec();

        for dataset in &side_datasets {
            self.split_and_downsample(dataset, asset_server, node_atlas);
        }

        self.task_queue.push_back(PreprocessTask::barrier());

        for lod in dataset.lod_range {
            for dataset in &side_datasets {
                self.stitch_and_save_layer(dataset, node_atlas, lod);
            }
        }

        self
    }
}

pub(crate) fn select_ready_tasks(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<(&mut Preprocessor, &mut NodeAtlas), With<Terrain>>,
) {
    for (mut preprocessor, mut node_atlas) in terrain_query.iter_mut() {
        let Preprocessor {
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

                node_atlas.save_node_config();
                // node_atlas.state.existing_nodes.iter().for_each(|node| {
                //     println!("{node}");
                // });

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

                // task.debug();

                if matches!(task.task_type, PreprocessTaskType::Save) {
                    node_atlas.save(task.node);
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
            println!("finished loading all tiles");
            preprocessor.loaded = true;
            preprocessor.start_time = Some(Instant::now());
        }
    }
}
