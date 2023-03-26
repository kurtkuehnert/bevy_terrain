use crate::{
    terrain::{Terrain, TerrainComponents},
    terrain_data::{
        node_atlas::{LoadingNode, NodeAtlas},
        AtlasAttachment, AtlasIndex,
    },
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
        Extract, MainWorld,
    },
};
use std::mem;

impl AtlasAttachment {
    /// Creates the attachment from its config.
    fn create(
        &self,
        device: &RenderDevice,
        images: &mut RenderAssets<Image>,
        node_atlas_size: AtlasIndex,
    ) -> Handle<Image> {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(&(self.name.to_string() + "_attachment")),
            size: Extent3d {
                width: self.texture_size,
                height: self.texture_size,
                depth_or_array_layers: node_atlas_size as u32,
            },
            mip_level_count: self.mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        images.insert(
            self.handle.clone(),
            GpuImage {
                texture_view: texture.create_view(&TextureViewDescriptor::default()),
                texture,
                texture_format: self.format,
                sampler: device.create_sampler(&SamplerDescriptor::default()),
                size: Vec2::splat(self.texture_size as f32),
                mip_level_count: self.mip_level_count,
            },
        );

        self.handle.clone()
    }
}

/// Stores the GPU representation of the [`NodeAtlas`] (array textures)
/// alongside the data to update it.
///
/// All attachments of newly loaded nodes are copied into their according atlas attachment.
#[derive(Component)]
pub struct GpuNodeAtlas {
    /// Stores the atlas attachments of the terrain.
    pub(crate) attachments: Vec<Handle<Image>>,
    /// Stores the nodes, that have finished loading this frame.
    pub(crate) loaded_nodes: Vec<LoadingNode>,
}

impl GpuNodeAtlas {
    /// Creates a new gpu node atlas and initializes its attachment textures.
    fn new(
        device: &RenderDevice,
        images: &mut RenderAssets<Image>,
        node_atlas: &NodeAtlas,
    ) -> Self {
        let attachments = node_atlas
            .attachments
            .iter()
            .map(|attachment| attachment.create(device, images, node_atlas.size))
            .collect();

        Self {
            attachments,
            loaded_nodes: Vec::new(),
        }
    }

    /// Updates the atlas attachments, by copying over the data of the nodes that have
    /// finished loading this frame.
    fn update(&mut self, command_encoder: &mut CommandEncoder, images: &RenderAssets<Image>) {
        for node in self.loaded_nodes.drain(..) {
            for (node_handle, atlas_handle) in
                self.attachments
                    .iter()
                    .enumerate()
                    .map(|(index, atlas_handle)| {
                        let node_handle = node.attachments.get(&index).unwrap();

                        (node_handle, atlas_handle)
                    })
            {
                if let (Some(node_attachment), Some(atlas_attachment)) =
                    (images.get(node_handle), images.get(atlas_handle))
                {
                    for mip_level in 0..node_attachment.mip_level_count {
                        // Todo: change to queue.write_texture
                        command_encoder.copy_texture_to_texture(
                            ImageCopyTexture {
                                texture: &node_attachment.texture,
                                mip_level,
                                origin: Origin3d { x: 0, y: 0, z: 0 },
                                aspect: TextureAspect::All,
                            },
                            ImageCopyTexture {
                                texture: &atlas_attachment.texture,
                                mip_level,
                                origin: Origin3d {
                                    x: 0,
                                    y: 0,
                                    z: node.atlas_index as u32,
                                },
                                aspect: TextureAspect::All,
                            },
                            Extent3d {
                                width: (node_attachment.size.x as u32) >> mip_level,
                                height: (node_attachment.size.y as u32) >> mip_level,
                                depth_or_array_layers: 1,
                            },
                        );
                    }
                } else {
                    error!("Something went wrong, attachment is not available!")
                }
            }
        }
    }
}

/// Initializes the [`GpuNodeAtlas`] of newly created terrains.
pub(crate) fn initialize_gpu_node_atlas(
    device: Res<RenderDevice>,
    mut images: ResMut<RenderAssets<Image>>,
    mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
    mut terrain_query: Extract<Query<(Entity, &NodeAtlas), Added<Terrain>>>,
) {
    for (terrain, node_atlas) in terrain_query.iter_mut() {
        gpu_node_atlases.insert(terrain, GpuNodeAtlas::new(&device, &mut images, node_atlas));
    }
}

/// Extracts the nodes that have finished loading from all [`NodeAtlas`]es into the
/// corresponding [`GpuNodeAtlas`]es.
pub(crate) fn extract_node_atlas(
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
pub(crate) fn prepare_node_atlas(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
    mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

    for terrain in terrain_query.iter() {
        let gpu_node_atlas = gpu_node_atlases.get_mut(&terrain).unwrap();
        gpu_node_atlas.update(&mut command_encoder, &images);
    }

    queue.submit(vec![command_encoder.finish()]);
}
