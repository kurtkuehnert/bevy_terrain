use crate::attachment_loader::{AttachmentFromDisk, AttachmentFromDiskLoader};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::GpuImage;
use bevy::utils::Uuid;
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::*},
    utils::HashMap,
};
use std::str::FromStr;

pub type TerrainComponents<C> = HashMap<Entity, C>;

#[derive(Clone, Copy, Component)]
pub struct Terrain;

impl ExtractComponent for Terrain {
    type Query = Read<Self>;
    type Filter = ();

    #[inline]
    fn extract_component(_item: QueryItem<Self::Query>) -> Self {
        Self
    }
}

pub type AttachmentIndex = usize;

/// Configures an [`AtlasAttachment`].
#[derive(Clone)]
pub struct AtlasAttachmentConfig {
    pub(crate) atlas_handle: Handle<Image>,
    pub(crate) texture_size: u32,
    pub(crate) border_size: u32,
    pub(crate) format: TextureFormat,
}

impl AtlasAttachmentConfig {
    /// Creates the attachment from its config.
    pub(crate) fn create(&self, config: &TerrainConfig, device: &RenderDevice) -> GpuImage {
        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: self.texture_size + 2 * self.border_size,
                height: self.texture_size + 2 * self.border_size,
                depth_or_array_layers: config.node_atlas_size as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        });
        let texture_view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor::default());

        GpuImage {
            texture,
            texture_view,
            texture_format: self.format,
            sampler,
            size: Vec2::splat((self.texture_size + 2 * self.border_size) as f32),
        }
    }
}

#[derive(Clone, Default, ShaderType)]
pub(crate) struct TerrainConfigUniform {
    lod_count: u32,
    height: f32,
    chunk_size: u32,
    _padding: u32,
    attachment_scales: Vec4,
    attachment_offsets: Vec4,
}

#[derive(Clone, Component)]
pub struct TerrainConfig {
    pub lod_count: u32,
    pub height: f32,
    pub chunk_size: u32,
    pub node_atlas_size: u16,
    pub path: String,
    pub attachments: Vec<AtlasAttachmentConfig>,
}

impl TerrainConfig {
    pub fn new(chunk_size: u32, lod_count: u32, height: f32, path: String) -> Self {
        let node_atlas_size = 1500;

        Self {
            lod_count,
            height,
            node_atlas_size,
            chunk_size,
            path,
            attachments: default(),
        }
    }

    pub fn add_attachment(
        &mut self,
        format: TextureFormat,
        texture_size: u32,
        border_size: u32,
    ) -> AttachmentIndex {
        // Todo: fix this awful hack
        let atlas_handle = HandleUntyped::weak_from_u64(
            Uuid::from_str("6ea26da6-6cf8-4ea2-9986-1d7bf6c17d6f").unwrap(),
            fastrand::u64(..),
        )
        .typed();

        self.attachments.push(AtlasAttachmentConfig {
            atlas_handle,
            texture_size,
            border_size,
            format,
        });

        self.attachments.len() - 1
    }

    pub fn add_attachment_from_disk(
        &mut self,
        from_disk_loader: &mut AttachmentFromDiskLoader,
        name: &str,
        format: TextureFormat,
        texture_size: u32,
        border_size: u32,
    ) {
        let attachment_index = self.add_attachment(format, texture_size, border_size);

        from_disk_loader.add_attachment(
            attachment_index,
            AttachmentFromDisk {
                path: self.path.clone() + "data/" + name,
                format,
            },
        );
    }

    pub(crate) fn shader_data(&self) -> TerrainConfigUniform {
        // Todo: figure out a better way to store data for more than four attachments
        let mut scales = [1.0; 4];
        let mut offsets = [0.0; 4];

        for (i, attachment) in self.attachments.iter().enumerate() {
            scales[i] = attachment.texture_size as f32
                / (attachment.texture_size + 2 * attachment.border_size) as f32;
            offsets[i] = attachment.border_size as f32
                / (attachment.texture_size + 2 * attachment.border_size) as f32;
        }

        TerrainConfigUniform {
            lod_count: self.lod_count,
            height: self.height,
            chunk_size: self.chunk_size,
            _padding: 0,
            attachment_scales: Vec4::from_array(scales),
            attachment_offsets: Vec4::from_array(offsets),
        }
    }
}
