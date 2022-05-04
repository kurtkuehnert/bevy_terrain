use crate::{
    config::TerrainConfig,
    node_atlas::NodeAtlas,
    quadtree::NodeData,
    render::{InitTerrain, PersistentComponent},
};
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice, RenderWorld},
    utils::HashMap,
};
use std::mem;

pub enum NodeAttachment {
    Buffer {
        binding: u32,
        buffer: Buffer,
    },
    Texture {
        view_binding: u32,
        sampler_binding: u32,
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
        texture_descriptor: TextureDescriptor<'static>,
        view_descriptor: TextureViewDescriptor<'static>,
        sampler_descriptor: SamplerDescriptor<'static>,
    },
}

pub struct GpuNodeAtlas {
    pub(crate) atlas_attachments: HashMap<String, NodeAttachment>,
    pub(crate) activated_nodes: Vec<(u16, NodeData)>, // make generic on NodeData
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
                        ref texture_descriptor,
                        ref view_descriptor,
                        ref sampler_descriptor,
                    } => {
                        let texture = device.create_texture(texture_descriptor);

                        NodeAttachment::Texture {
                            view_binding,
                            sampler_binding,
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

/// Runs in prepare.
pub(crate) fn init_gpu_node_atlas(
    device: Res<RenderDevice>,
    mut gpu_node_atlases: ResMut<PersistentComponent<GpuNodeAtlas>>,
    terrain_query: Query<(Entity, &TerrainConfig), With<InitTerrain>>,
) {
    for (entity, config) in terrain_query.iter() {
        info!("initializing gpu node atlas");

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

        gpu_node_atlas.activated_nodes = mem::take(&mut node_atlas.activated_nodes);
    }
}
