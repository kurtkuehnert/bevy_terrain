use crate::{
    config::TerrainConfig,
    node_atlas::{NodeAtlas, NodeAttachment, NodeData},
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

/// A modular resource of the node atlas, that can be accessed in the terrain shader.
/// It may store data (buffer/texture) for each active node or a sampler.
pub enum AtlasAttachment {
    /// An (array) buffer with data per active node.
    Buffer {
        binding: u32, // Todo: rework the ordering
        buffer: Buffer,
    },
    /// An array texture with data per active node.
    Texture {
        binding: u32,
        texture_size: u32,
        texture: Texture,
        view: TextureView,
    },
    /// A sampler used in conjunction with one or more texture attachments.
    Sampler { binding: u32, sampler: Sampler },
}

impl AtlasAttachment {
    /// Returns the binding of the attachment.
    pub(crate) fn binding(&self) -> BindGroupEntry {
        match self {
            &AtlasAttachment::Buffer {
                binding,
                ref buffer,
            } => BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            },
            &AtlasAttachment::Texture {
                binding, ref view, ..
            } => BindGroupEntry {
                binding,
                resource: BindingResource::TextureView(view),
            },
            &AtlasAttachment::Sampler {
                binding,
                ref sampler,
            } => BindGroupEntry {
                binding,
                resource: BindingResource::Sampler(sampler),
            },
        }
    }

    /// Returns the bind group layout of the attachment.
    pub(crate) fn layout(&self) -> BindGroupLayoutEntry {
        match self {
            &AtlasAttachment::Buffer { binding, .. } => BindGroupLayoutEntry {
                binding,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            &AtlasAttachment::Texture { binding, .. } => BindGroupLayoutEntry {
                binding,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            &AtlasAttachment::Sampler { binding, .. } => BindGroupLayoutEntry {
                binding,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        }
    }
}

/// Configures an [`AtlasAttachment`].
#[derive(Clone)]
pub enum AtlasAttachmentConfig {
    /// An (array) buffer with data per active node.
    Buffer {
        binding: u32,
        descriptor: BufferDescriptor<'static>,
    },
    /// An array texture with data per active node.
    Texture {
        binding: u32,
        texture_size: u32,
        texture_descriptor: TextureDescriptor<'static>,
        view_descriptor: TextureViewDescriptor<'static>,
    },
    /// A sampler used in conjunction with one or more texture attachments.
    Sampler {
        binding: u32,
        sampler_descriptor: SamplerDescriptor<'static>,
    },
}

impl AtlasAttachmentConfig {
    /// Creates the attachment from its config.
    pub(crate) fn create(&self, device: &RenderDevice) -> AtlasAttachment {
        match self {
            &AtlasAttachmentConfig::Buffer {
                binding,
                ref descriptor,
            } => AtlasAttachment::Buffer {
                binding,
                buffer: device.create_buffer(descriptor),
            },
            &AtlasAttachmentConfig::Texture {
                binding,
                texture_size,
                ref texture_descriptor,
                ref view_descriptor,
            } => {
                let texture = device.create_texture(texture_descriptor);

                AtlasAttachment::Texture {
                    binding,
                    texture_size,
                    view: texture.create_view(view_descriptor),
                    texture,
                }
            }
            &AtlasAttachmentConfig::Sampler {
                binding,
                ref sampler_descriptor,
            } => AtlasAttachment::Sampler {
                binding,
                sampler: device.create_sampler(sampler_descriptor),
            },
        }
    }
}

/// Manages the [`AtlasAttachment`]s of the terrain, by updating them with the data of
/// the [`NodeAttachment`]s of newly activated nodes.
#[derive(Component)]
pub struct GpuNodeAtlas {
    pub(crate) atlas_attachments: HashMap<String, AtlasAttachment>,
    pub(crate) activated_nodes: Vec<NodeData>, // Todo: consider own component
}

impl GpuNodeAtlas {
    fn new(config: &TerrainConfig, device: &RenderDevice) -> Self {
        let atlas_attachments = config
            .attachments
            .iter()
            .map(|(label, attachment_config)| (label.clone(), attachment_config.create(device)))
            .collect();

        Self {
            atlas_attachments,
            activated_nodes: Vec::new(),
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

        gpu_node_atlas.activated_nodes = mem::take(&mut node_atlas.activated_nodes);
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

        for node_data in &gpu_node_atlas.activated_nodes {
            for (data, texture, texture_size) in
                gpu_node_atlas
                    .atlas_attachments
                    .iter()
                    .filter_map(|(label, attachment)| {
                        let node_attachment = node_data.attachment_data.get(label)?;

                        match (node_attachment, attachment) {
                            (NodeAttachment::Buffer { .. }, AtlasAttachment::Buffer { .. }) => None,
                            (
                                NodeAttachment::Texture { handle: data },
                                &AtlasAttachment::Texture {
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
