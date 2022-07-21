use crate::{
    attachment_loader::AttachmentFromDiskLoader,
    data_structures::{AtlasAttachment, AttachmentIndex},
};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::*},
    utils::HashMap,
    utils::Uuid,
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

#[derive(Clone, Default, ShaderType)]
pub(crate) struct TerrainConfigUniform {
    lod_count: u32,
    height: f32,
    chunk_size: u32,
    terrain_size: u32,
    attachment_scales: Vec4,
    attachment_offsets: Vec4,
}

#[derive(Clone, Component)]
pub struct TerrainConfig {
    pub lod_count: u32,
    pub height: f32,
    pub chunk_size: u32,
    pub terrain_size: u32,
    pub node_atlas_size: u32,
    pub path: String,
    pub attachments: Vec<AtlasAttachment>,
}

impl TerrainConfig {
    pub fn new(
        terrain_size: u32,
        chunk_size: u32,
        lod_count: u32,
        height: f32,
        node_atlas_size: u32,
        path: String,
    ) -> Self {
        Self {
            lod_count,
            height,
            node_atlas_size,
            chunk_size,
            terrain_size,
            path,
            attachments: default(),
        }
    }

    pub fn add_attachment(
        &mut self,
        name: &'static str,
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

        self.attachments.push(AtlasAttachment {
            name,
            handle: atlas_handle,
            texture_size,
            border_size,
            format,
        });

        self.attachments.len() - 1
    }

    pub fn add_attachment_from_disk(
        &mut self,
        from_disk_loader: &mut AttachmentFromDiskLoader,
        name: &'static str,
        format: TextureFormat,
        texture_size: u32,
        border_size: u32,
    ) {
        let attachment_index = self.add_attachment(name, format, texture_size, border_size);

        from_disk_loader.add_attachment(
            attachment_index,
            self.path.clone() + "data/" + name,
            format,
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
            terrain_size: self.terrain_size,
            attachment_scales: Vec4::from_array(scales),
            attachment_offsets: Vec4::from_array(offsets),
        }
    }
}
