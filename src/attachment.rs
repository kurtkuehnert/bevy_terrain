use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
};

pub type AttachmentIndex = u32; // Todo: decide mapping between attachment index and binding

/// A modular resource of the node atlas, that can be accessed in the terrain shader.
/// It may store data (buffer/texture) for each active node or a sampler.
pub enum AtlasAttachment {
    /// An (array) buffer with data per active node.
    Buffer { buffer: Buffer },
    /// An array texture with data per active node.
    Texture {
        texture_size: u32,
        texture: Texture,
        view: TextureView,
    },
    /// A sampler used in conjunction with one or more texture attachments.
    Sampler { sampler: Sampler },
}

impl AtlasAttachment {
    /// Returns the bind group entry of the attachment.
    pub(crate) fn bind_group_entry(&self, binding: u32) -> BindGroupEntry {
        match self {
            &AtlasAttachment::Buffer { ref buffer } => BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            },
            &AtlasAttachment::Texture { ref view, .. } => BindGroupEntry {
                binding,
                resource: BindingResource::TextureView(view),
            },
            &AtlasAttachment::Sampler { ref sampler } => BindGroupEntry {
                binding,
                resource: BindingResource::Sampler(sampler),
            },
        }
    }

    /// Returns the bind group layout entry of the attachment.
    pub(crate) fn layout_entry(&self, binding: u32) -> BindGroupLayoutEntry {
        match self {
            &AtlasAttachment::Buffer { .. } => BindGroupLayoutEntry {
                binding,
                visibility: ShaderStages::all(),
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            &AtlasAttachment::Texture { .. } => BindGroupLayoutEntry {
                binding,
                visibility: ShaderStages::all(),
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            &AtlasAttachment::Sampler { .. } => BindGroupLayoutEntry {
                binding,
                visibility: ShaderStages::all(),
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
        descriptor: BufferDescriptor<'static>,
    },
    /// An array texture with data per active node.
    Texture {
        texture_size: u32,
        texture_descriptor: TextureDescriptor<'static>,
        view_descriptor: TextureViewDescriptor<'static>,
    },
    /// A sampler used in conjunction with one or more texture attachments.
    Sampler {
        sampler_descriptor: SamplerDescriptor<'static>,
    },
}

impl AtlasAttachmentConfig {
    /// Creates the attachment from its config.
    pub(crate) fn create(&self, device: &RenderDevice) -> AtlasAttachment {
        match self {
            &AtlasAttachmentConfig::Buffer { ref descriptor } => AtlasAttachment::Buffer {
                buffer: device.create_buffer(descriptor),
            },
            &AtlasAttachmentConfig::Texture {
                texture_size,
                ref texture_descriptor,
                ref view_descriptor,
            } => {
                let texture = device.create_texture(texture_descriptor);

                AtlasAttachment::Texture {
                    texture_size,
                    view: texture.create_view(view_descriptor),
                    texture,
                }
            }
            &AtlasAttachmentConfig::Sampler {
                ref sampler_descriptor,
            } => AtlasAttachment::Sampler {
                sampler: device.create_sampler(sampler_descriptor),
            },
        }
    }
}

/// Stores the data, which will be loaded into the corresponding [`AtlasAttachment`] once the node
/// becomes activated.
#[derive(Clone)]
pub enum NodeAttachment {
    Buffer { data: Vec<u8> },
    Texture { handle: Handle<Image> },
}
