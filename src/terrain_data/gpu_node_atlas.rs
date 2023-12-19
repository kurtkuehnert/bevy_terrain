use crate::terrain_data::node_atlas::ReadBackNode;
use crate::{
    terrain::{Terrain, TerrainComponents},
    terrain_data::{
        gpu_atlas_attachment::GpuAtlasAttachment,
        node_atlas::{LoadingNode, NodeAtlas},
    },
};
use bevy::tasks::Task;
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        Extract, MainWorld,
    },
};
use std::mem;
use std::sync::{Arc, Mutex};

/// Stores the GPU representation of the [`NodeAtlas`] (array textures)
/// alongside the data to update it.
///
/// All attachments of newly loaded nodes are copied into their according atlas attachment.
#[derive(Component)]
pub struct GpuNodeAtlas {
    /// Stores the atlas attachments of the terrain.
    pub(crate) attachments: Vec<GpuAtlasAttachment>,
    /// Stores the nodes, that have finished loading this frame.
    pub(crate) loaded_nodes: Vec<LoadingNode>,
    pub(crate) read_back_nodes: Arc<Mutex<Vec<Task<Vec<ReadBackNode>>>>>,
}

impl GpuNodeAtlas {
    /// Creates a new gpu node atlas and initializes its attachment textures.
    fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        images: &mut RenderAssets<Image>,
        node_atlas: &NodeAtlas,
    ) -> Self {
        let attachments = node_atlas
            .attachments
            .iter()
            .map(|attachment| {
                GpuAtlasAttachment::create(attachment, device, queue, images, node_atlas.size)
            })
            .collect::<Vec<_>>();

        Self {
            attachments,
            loaded_nodes: default(),
            read_back_nodes: node_atlas.read_back_nodes.clone(),
        }
    }

    /// Updates the atlas attachments, by copying over the data of the nodes that have
    /// finished loading this frame.
    fn update_attachments(
        &mut self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
    ) {
        for node in self.loaded_nodes.drain(..) {
            for (node_handle, attachment) in
                self.attachments
                    .iter()
                    .enumerate()
                    .map(|(index, atlas_handle)| {
                        let node_handle = node.attachments.get(&index).unwrap();

                        (node_handle, atlas_handle)
                    })
            {
                attachment.upload_node(command_encoder, images, node_handle, node.atlas_index);
            }
        }
    }

    /// Initializes the [`GpuNodeAtlas`] of newly created terrains.
    pub(crate) fn initialize(
        device: Res<RenderDevice>,
        queue: Res<RenderQueue>,
        mut images: ResMut<RenderAssets<Image>>,
        mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
        mut terrain_query: Extract<Query<(Entity, &NodeAtlas), Added<Terrain>>>,
    ) {
        for (terrain, node_atlas) in terrain_query.iter_mut() {
            gpu_node_atlases.insert(
                terrain,
                GpuNodeAtlas::new(&device, &queue, &mut images, node_atlas),
            );
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
                &mut node_atlas.loaded_nodes,
                &mut gpu_node_atlas.loaded_nodes,
            );
        }
    }

    /// Queues the attachments of the nodes that have finished loading to be copied into the
    /// corresponding atlas attachments.
    pub(crate) fn prepare(
        device: Res<RenderDevice>,
        queue: Res<RenderQueue>,
        images: Res<RenderAssets<Image>>,
        mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        let mut command_encoder =
            device.create_command_encoder(&CommandEncoderDescriptor::default());

        for terrain in terrain_query.iter() {
            let gpu_node_atlas = gpu_node_atlases.get_mut(&terrain).unwrap();
            gpu_node_atlas.update_attachments(&mut command_encoder, &images);

            let attachment = &mut gpu_node_atlas.attachments[0];

            attachment.create_read_back_buffer(&device);
        }

        queue.submit(vec![command_encoder.finish()]);
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
