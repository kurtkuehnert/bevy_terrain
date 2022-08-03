use crate::{
    attachment_loader::{AttachmentFromDisk, AttachmentFromDiskLoader},
    preprocess::{BaseConfig, Preprocessor, TileConfig},
    terrain_data::{AtlasAttachment, AttachmentConfig, AttachmentFormat, AttachmentIndex},
};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::*},
    utils::HashMap,
};

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

    pub fn add_attachment(&mut self, attachment: AttachmentConfig) -> AttachmentIndex {
        self.attachments.push(attachment.into());
        self.attachments.len() - 1
    }

    pub fn add_base_attachment(
        &mut self,
        preprocessor: &mut Preprocessor,
        from_disk_loader: &mut AttachmentFromDiskLoader,
        center_size: u32,
        tile: TileConfig,
    ) {
        let height_attachment = AttachmentConfig {
            name: "height".to_string(),
            center_size,
            border_size: 2,
            format: AttachmentFormat::LUMA16,
        };
        let density_attachment = AttachmentConfig {
            name: "density".to_string(),
            center_size,
            border_size: 0,
            format: AttachmentFormat::LUMA16,
        };

        preprocessor.base = (tile, BaseConfig { center_size });

        from_disk_loader.attachments.insert(
            self.attachments.len(),
            AttachmentFromDisk {
                path: format!("{}/data/{}", self.path, height_attachment.name),
                format: AttachmentFormat::LUMA16.into(),
            },
        );

        self.attachments.push(height_attachment.into());

        from_disk_loader.attachments.insert(
            self.attachments.len(),
            AttachmentFromDisk {
                path: format!("{}/data/{}", self.path, density_attachment.name),
                format: AttachmentFormat::LUMA16.into(),
            },
        );

        self.attachments.push(density_attachment.into());
    }

    pub fn add_attachment_from_disk(
        &mut self,
        preprocessor: &mut Preprocessor,
        from_disk_loader: &mut AttachmentFromDiskLoader,
        attachment: AttachmentConfig,
        tile: TileConfig,
    ) {
        let attachment_index = self.add_attachment(attachment.clone());

        from_disk_loader.attachments.insert(
            attachment_index,
            AttachmentFromDisk {
                path: format!("{}/data/{}", self.path, attachment.name),
                format: attachment.format.into(),
            },
        );

        preprocessor.attachments.push((tile, attachment));
    }

    pub(crate) fn shader_data(&self) -> TerrainConfigUniform {
        // Todo: figure out a better way to store data for more than four attachments
        let mut scales = [1.0; 4];
        let mut offsets = [0.0; 4];

        for (i, attachment) in self.attachments.iter().enumerate() {
            scales[i] = attachment.center_size as f32
                / (attachment.center_size + 2 * attachment.border_size) as f32;
            offsets[i] = attachment.border_size as f32
                / (attachment.center_size + 2 * attachment.border_size) as f32;
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
