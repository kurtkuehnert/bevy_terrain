use crate::node_atlas::NodeData;
use crate::{
    config::TerrainConfig,
    node_atlas::NodeAtlas,
    render::{InitTerrain, PersistentComponent},
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets, render_resource::*, renderer::RenderDevice,
        renderer::RenderQueue, RenderWorld,
    },
    utils::HashMap,
};
use std::mem;

#[derive(Clone)]
pub enum NodeAttachmentData {
    Buffer { data: Vec<u8> },
    Texture { data: Handle<Image> },
}

pub enum NodeAttachment {
    Buffer {
        binding: u32,
        buffer: Buffer,
    },
    Texture {
        view_binding: u32,
        sampler_binding: u32,
        texture_size: u32,
        texture: Texture,
        view: TextureView,
        sampler: Sampler,
    },
}

#[derive(Clone)]
pub enum NodeAttachmentConfig {
    Buffer {
        binding: u32,
        descriptor: BufferDescriptor<'static>,
    },
    Texture {
        view_binding: u32,
        sampler_binding: u32,
        texture_size: u32,
        texture_descriptor: TextureDescriptor<'static>,
        view_descriptor: TextureViewDescriptor<'static>,
        sampler_descriptor: SamplerDescriptor<'static>,
    },
}

pub struct GpuNodeAtlas {
    pub(crate) atlas_attachments: HashMap<String, NodeAttachment>,
    pub(crate) activated_nodes: Vec<NodeData>, // make generic on NodeData
}

impl GpuNodeAtlas {
    fn new(config: &TerrainConfig, device: &RenderDevice) -> Self {
        let atlas_attachments = config
            .node_attachment_configs
            .as_ref()
            .unwrap()
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

/// Initializes the [`GpuNodeAtlas`] of newly created terrains. Runs during the [`Prepare`](bevy::render::RenderStage::Prepare) stage.
pub(crate) fn init_gpu_node_atlas(
    device: Res<RenderDevice>,
    mut gpu_node_atlases: ResMut<PersistentComponent<GpuNodeAtlas>>,
    terrain_query: Query<(Entity, &TerrainConfig), With<InitTerrain>>,
) {
    for (entity, config) in terrain_query.iter() {
        gpu_node_atlases.insert(entity, GpuNodeAtlas::new(config, &device));
    }
}

pub(crate) fn extract_node_atlas(
    mut render_world: ResMut<RenderWorld>,
    mut terrain_query: Query<(Entity, &mut NodeAtlas), With<TerrainConfig>>,
) {
    let mut gpu_node_atlases = render_world.resource_mut::<PersistentComponent<GpuNodeAtlas>>();

    for (entity, mut node_atlas) in terrain_query.iter_mut() {
        let gpu_node_atlas = match gpu_node_atlases.get_mut(&entity) {
            Some(gpu_node_atlas) => gpu_node_atlas,
            None => continue,
        };

        // node_atlas
        //     .active_nodes
        //     .extend(mem::take(&mut gpu_node_atlas.activated_nodes).into_iter());
        gpu_node_atlas.activated_nodes = mem::take(&mut node_atlas.activated_nodes);
    }
}

pub(crate) fn queue_node_attachment_updates(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
    mut gpu_node_atlases: ResMut<PersistentComponent<GpuNodeAtlas>>,
    terrain_query: Query<Entity, With<TerrainConfig>>,
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
                        let node_attachment = node_data.node_attachments.get(label).unwrap();

                        match (node_attachment, attachment) {
                            (NodeAttachmentData::Buffer { .. }, NodeAttachment::Buffer { .. }) => {
                                None
                            }
                            (
                                NodeAttachmentData::Texture { data },
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
