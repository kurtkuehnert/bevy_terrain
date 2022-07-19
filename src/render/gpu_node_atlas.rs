use crate::{
    node_atlas::{LoadingNode, NodeAtlas},
    terrain::{AttachmentIndex, Terrain, TerrainComponents, TerrainConfig},
};
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

/// Manages the [`AtlasAttachment`]s of the terrain, by updating them with the data of
/// the [`NodeAttachment`]s of newly activated nodes.
#[derive(Component)]
pub struct GpuNodeAtlas {
    pub(crate) atlas_attachments: Vec<Handle<Image>>,
    pub(crate) loaded_nodes: Vec<LoadingNode>,
}

impl GpuNodeAtlas {
    fn new(
        config: &TerrainConfig, // Todo: change to NodeAtlas
        device: &RenderDevice,
        images: &mut RenderAssets<Image>,
    ) -> Self {
        let atlas_attachments = config
            .attachments
            .iter()
            .map(|attachment_config| {
                images.insert(
                    attachment_config.atlas_handle.clone(),
                    attachment_config.create(config, device),
                );

                attachment_config.atlas_handle.clone()
            })
            .collect();

        Self {
            atlas_attachments,
            loaded_nodes: Vec::new(),
        }
    }

    fn update(&mut self, command_encoder: &mut CommandEncoder, images: &RenderAssets<Image>) {
        for node in self.loaded_nodes.drain(..) {
            for (node_handle, atlas_handle) in
                self.atlas_attachments
                    .iter()
                    .enumerate()
                    .map(|(index, atlas_handle)| {
                        let node_handle =
                            node.attachments.get(&(index as AttachmentIndex)).unwrap();

                        (node_handle, atlas_handle)
                    })
            {
                if let (Some(node_attachment), Some(atlas_attachment)) =
                    (images.get(node_handle), images.get(atlas_handle))
                {
                    // Todo: change to queue.write_texture
                    command_encoder.copy_texture_to_texture(
                        ImageCopyTexture {
                            texture: &node_attachment.texture,
                            mip_level: 0,
                            origin: Origin3d { x: 0, y: 0, z: 0 },
                            aspect: TextureAspect::All,
                        },
                        ImageCopyTexture {
                            texture: &atlas_attachment.texture,
                            mip_level: 0,
                            origin: Origin3d {
                                x: 0,
                                y: 0,
                                z: node.atlas_index as u32,
                            },
                            aspect: TextureAspect::All,
                        },
                        Extent3d {
                            width: atlas_attachment.size.x as u32,
                            height: atlas_attachment.size.y as u32,
                            depth_or_array_layers: 1,
                        },
                    );
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
    mut terrain_query: Extract<Query<(Entity, &TerrainConfig), Added<Terrain>>>,
) {
    for (terrain, config) in terrain_query.iter_mut() {
        gpu_node_atlases.insert(terrain, GpuNodeAtlas::new(config, &device, &mut images));
    }
}

/// Updates the [`GpuNodeAtlas`] with the activated nodes of the current frame.
pub(crate) fn update_gpu_node_atlas(
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

/// Updates the [`AtlasAttachment`]s of the terrain, by updating them with the data of
/// the [`NodeAttachment`]s of activated nodes.
pub(crate) fn queue_node_atlas_updates(
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
