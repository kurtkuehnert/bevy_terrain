use crate::{
    terrain::{Terrain, TerrainComponents},
    terrain_data::{
        gpu_atlas_attachment::GpuAtlasAttachment,
        node_atlas::{NodeAtlas, NodeWithData},
    },
};
use bevy::{
    prelude::*,
    render::{
        renderer::{RenderDevice, RenderQueue},
        Extract, MainWorld,
    },
    tasks::Task,
};
use itertools::Itertools;
use std::{
    mem,
    sync::{Arc, Mutex},
};

/// Stores the GPU representation of the [`NodeAtlas`] (array textures)
/// alongside the data to update it.
///
/// All attachments of newly loaded nodes are copied into their according atlas attachment.
#[derive(Component)]
pub struct GpuNodeAtlas {
    /// Stores the atlas attachments of the terrain.
    pub(crate) attachments: Vec<GpuAtlasAttachment>,
    /// Stores the nodes, that have finished loading this frame.
    pub(crate) loaded_nodes: Vec<NodeWithData>,
    pub(crate) read_back_nodes: Arc<Mutex<Vec<Task<Vec<NodeWithData>>>>>,
}

impl GpuNodeAtlas {
    /// Creates a new gpu node atlas and initializes its attachment textures.
    fn new(device: &RenderDevice, node_atlas: &NodeAtlas) -> Self {
        let attachments = node_atlas
            .attachments
            .iter()
            .map(|attachment| GpuAtlasAttachment::create(attachment, device, node_atlas.size))
            .collect_vec();

        Self {
            attachments,
            loaded_nodes: default(),
            read_back_nodes: node_atlas.read_back_nodes.clone(),
        }
    }

    /// Updates the atlas attachments, by copying over the data of the nodes that have
    /// finished loading this frame.
    fn update(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let attachment = &mut self.attachments[0];

        attachment.create_read_back_buffer(&device);

        for node in self.loaded_nodes.drain(..) {
            attachment.upload_node(queue, node);
        }
    }

    /// Initializes the [`GpuNodeAtlas`] of newly created terrains.
    pub(crate) fn initialize(
        device: Res<RenderDevice>,
        mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
        mut terrain_query: Extract<Query<(Entity, &NodeAtlas), Added<Terrain>>>,
    ) {
        for (terrain, node_atlas) in terrain_query.iter_mut() {
            gpu_node_atlases.insert(terrain, GpuNodeAtlas::new(&device, node_atlas));
        }
    }

    /// Extracts the nodes that have finished loading from all [`NodeAtlas`]es into the
    /// corresponding [`GpuNodeAtlas`]es.
    pub(crate) fn extract(
        mut main_world: ResMut<MainWorld>,
        mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
    ) {
        let mut terrain_query = main_world.query::<(Entity, &mut NodeAtlas)>();

        for (terrain, mut node_atlas) in terrain_query.iter_mut(&mut main_world) {
            let gpu_node_atlas = gpu_node_atlases.get_mut(&terrain).unwrap();
            mem::swap(
                &mut node_atlas.finished_loading_nodes,
                &mut gpu_node_atlas.loaded_nodes,
            );
        }
    }

    /// Queues the attachments of the nodes that have finished loading to be copied into the
    /// corresponding atlas attachments.
    pub(crate) fn prepare(
        device: Res<RenderDevice>,
        queue: Res<RenderQueue>,
        mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        for terrain in terrain_query.iter() {
            let gpu_node_atlas = gpu_node_atlases.get_mut(&terrain).unwrap();
            gpu_node_atlas.update(&device, &queue);
        }
    }

    pub(crate) fn cleanup(
        mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        for terrain in terrain_query.iter() {
            let gpu_node_atlas = gpu_node_atlases.get_mut(&terrain).unwrap();
            let attachment = &mut gpu_node_atlas.attachments[0];

            if !attachment.slots.is_empty() {
                gpu_node_atlas
                    .read_back_nodes
                    .lock()
                    .unwrap()
                    .push(attachment.start_reading_back_nodes());
            }
        }
    }
}
