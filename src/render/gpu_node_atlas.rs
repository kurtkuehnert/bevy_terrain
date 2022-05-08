use crate::{
    attachments::{NodeAttachment, NodeAttachmentConfig, NodeAttachmentData},
    config::TerrainConfig,
    node_atlas::{NodeAtlas, NodeData},
    persistent_component::PersistentComponent,
    PersistentComponents, Terrain,
};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_component::ExtractComponent,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        RenderWorld,
    },
    utils::HashMap,
};
use std::mem;

#[derive(Component)]
pub struct GpuNodeAtlas {
    pub(crate) atlas_attachments: HashMap<String, NodeAttachment>,
    pub(crate) activated_nodes: Vec<NodeData>,
}

impl GpuNodeAtlas {
    fn new(config: &TerrainConfig, device: &RenderDevice) -> Self {
        let atlas_attachments = config
            .attachments
            .iter()
            .map(|(label, attachment_config)| {
                let attachment = match attachment_config {
                    &NodeAttachmentConfig::Buffer {
                        binding,
                        ref descriptor,
                    } => NodeAttachment::Buffer {
                        binding,
                        buffer: device.create_buffer(descriptor),
                    },
                    &NodeAttachmentConfig::Texture {
                        view_binding,
                        sampler_binding,
                        texture_size,
                        ref texture_descriptor,
                        ref view_descriptor,
                        ref sampler_descriptor,
                    } => {
                        let texture = device.create_texture(texture_descriptor);

                        NodeAttachment::Texture {
                            view_binding,
                            sampler_binding,
                            texture_size,
                            view: texture.create_view(view_descriptor),
                            sampler: device.create_sampler(sampler_descriptor),
                            texture,
                        }
                    }
                };

                (label.clone(), attachment)
            })
            .collect();

        Self {
            atlas_attachments,
            activated_nodes: Vec::new(),
        }
    }
}

pub(crate) fn initialize_gpu_node_atlas(
    mut components: ResMut<PersistentComponents<GpuNodeAtlas>>,
    device: Res<RenderDevice>,
    mut terrain_query: Query<(Entity, &TerrainConfig)>,
) {
    for (entity, config) in terrain_query.iter_mut() {
        components.insert(entity, GpuNodeAtlas::new(config, &device));
    }
}

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

        gpu_node_atlas.activated_nodes = mem::take(&mut node_atlas.activated_nodes);
    }
}

pub(crate) fn queue_node_attachment_updates(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
    mut gpu_node_atlases: ResMut<PersistentComponents<GpuNodeAtlas>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

    for entity in terrain_query.iter() {
        let gpu_node_atlas = gpu_node_atlases.get_mut(&entity).unwrap();

        for node_data in &gpu_node_atlas.activated_nodes {
            for (data, texture, texture_size) in
                gpu_node_atlas
                    .atlas_attachments
                    .iter()
                    .filter_map(|(label, attachment)| {
                        let node_attachment = node_data.attachment_data.get(label).unwrap();

                        match (node_attachment, attachment) {
                            (NodeAttachmentData::Buffer { .. }, NodeAttachment::Buffer { .. }) => {
                                None
                            }
                            (
                                NodeAttachmentData::Texture { handle: data },
                                &NodeAttachment::Texture {
                                    ref texture,
                                    texture_size,
                                    ..
                                },
                            ) => Some((data, texture, texture_size)),
                            _ => None,
                        }
                    })
            {
                let image = images.get(data).unwrap();

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
                            z: node_data.atlas_index as u32,
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
