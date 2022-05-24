use crate::{
    attachment::{AtlasAttachment, AttachmentIndex, NodeAttachment},
    config::TerrainConfig,
    node_atlas::{LoadingNode, NodeAtlas},
    render::PersistentComponents,
    Terrain,
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        RenderWorld,
    },
    utils::HashMap,
};
use std::mem;

/// Manages the [`AtlasAttachment`]s of the terrain, by updating them with the data of
/// the [`NodeAttachment`]s of newly activated nodes.
#[derive(Component)]
pub struct GpuNodeAtlas {
    pub(crate) atlas_attachments: HashMap<AttachmentIndex, AtlasAttachment>,
    pub(crate) loaded_nodes: Vec<LoadingNode>, // Todo: consider own component
}

impl GpuNodeAtlas {
    fn new(config: &TerrainConfig, device: &RenderDevice) -> Self {
        let atlas_attachments = config
            .attachments
            .iter()
            .map(|(attachment_index, attachment_config)| {
                (attachment_index.clone(), attachment_config.create(device))
            })
            .collect();

        Self {
            atlas_attachments,
            loaded_nodes: Vec::new(),
        }
    }
}

/// Initializes the [`GpuNodeAtlas`] of newly created terrains.
pub(crate) fn initialize_gpu_node_atlas(
    mut components: ResMut<PersistentComponents<GpuNodeAtlas>>,
    device: Res<RenderDevice>,
    mut terrain_query: Query<(Entity, &TerrainConfig)>,
) {
    for (entity, config) in terrain_query.iter_mut() {
        components.insert(entity, GpuNodeAtlas::new(config, &device));
    }
}

/// Updates the [`GpuNodeAtlas`] with the activated nodes of the current frame.
pub(crate) fn update_gpu_node_atlas(
    mut render_world: ResMut<RenderWorld>,
    mut terrain_query: Query<(Entity, &mut NodeAtlas)>,
) {
    let mut components = render_world.resource_mut::<PersistentComponents<GpuNodeAtlas>>();

    for (entity, mut node_atlas) in terrain_query.iter_mut() {
        let gpu_node_atlas = match components.get_mut(&entity) {
            Some(component) => component,
            None => continue,
        };

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
    mut gpu_node_atlases: ResMut<PersistentComponents<GpuNodeAtlas>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

    for entity in terrain_query.iter() {
        let gpu_node_atlas = gpu_node_atlases.get_mut(&entity).unwrap();

        for node in gpu_node_atlas.loaded_nodes.drain(..) {
            for (handle, texture, texture_size) in gpu_node_atlas
                .atlas_attachments
                .iter()
                .filter_map(|(attachment_index, attachment)| {
                    let node_attachment = node.attachments.get(attachment_index)?;

                    match (node_attachment, attachment) {
                        (NodeAttachment::Buffer { .. }, AtlasAttachment::Buffer { .. }) => None,
                        (
                            NodeAttachment::Texture { handle },
                            &AtlasAttachment::Texture {
                                ref texture,
                                texture_size,
                                ..
                            },
                        ) => Some((handle, texture, texture_size)),
                        _ => None,
                    }
                })
            {
                let image = images.get(handle).unwrap(); // Todo: investigate this occasional panic

                command_encoder.copy_texture_to_texture(
                    ImageCopyTexture {
                        texture: &image.texture,
                        mip_level: 0,
                        origin: Origin3d { x: 0, y: 0, z: 0 },
                        aspect: TextureAspect::All,
                    },
                    ImageCopyTexture {
                        texture,
                        mip_level: 0,
                        origin: Origin3d {
                            x: 0,
                            y: 0,
                            z: node.atlas_index as u32,
                        },
                        aspect: TextureAspect::All,
                    },
                    Extent3d {
                        width: texture_size,
                        height: texture_size,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
    }

    queue.submit(vec![command_encoder.finish()]);
}
