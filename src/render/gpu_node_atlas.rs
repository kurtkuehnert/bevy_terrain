use crate::{
    attachments::{NodeAttachment, NodeAttachmentConfig, NodeAttachmentData},
    config::TerrainConfig,
    node_atlas::{NodeAtlas, NodeData},
    persistent_component::PersistentComponent,
    PersistentComponents,
};
use bevy::{
    ecs::{
        query::QueryItem,
        system::{
            lifetimeless::{Read, SRes, Write},
            SystemParamItem,
        },
    },
    prelude::*,
    render::{
        render_asset::RenderAssets, render_resource::*, renderer::RenderDevice,
        renderer::RenderQueue,
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

impl PersistentComponent for GpuNodeAtlas {
    type InsertFilter = Added<NodeAtlas>;
    type InitializeQuery = Read<TerrainConfig>;
    type InitializeParam = SRes<RenderDevice>;
    type UpdateQuery = Write<NodeAtlas>;
    type UpdateFilter = ();

    /// Initializes the [`GpuNodeAtlas`] in the render world
    /// once a [`NodeAtlas`] is added to an entity in the app world.
    fn initialize_component(
        config: QueryItem<Self::InitializeQuery>,
        device: &mut SystemParamItem<Self::InitializeParam>,
    ) -> Self {
        Self::new(config, &device)
    }

    fn update_component(&mut self, mut node_atlas: QueryItem<Self::UpdateQuery>) {
        self.activated_nodes = mem::take(&mut node_atlas.activated_nodes);
    }
}

pub(crate) fn queue_node_attachment_updates(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
    mut gpu_node_atlases: ResMut<PersistentComponents<GpuNodeAtlas>>,
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
