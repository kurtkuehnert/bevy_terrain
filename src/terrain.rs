use crate::attachment::{AtlasAttachmentConfig, AttachmentIndex};
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
}

#[derive(Clone, Component)]
pub struct TerrainConfig {
    pub lod_count: u32,
    pub height: f32,
    pub chunk_size: u32,
    pub node_atlas_size: u16,
    pub path: String,
    pub attachments: HashMap<AttachmentIndex, AtlasAttachmentConfig>,
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
        attachment_index: AttachmentIndex,
        attachment_config: AtlasAttachmentConfig,
    ) {
        self.attachments.insert(attachment_index, attachment_config);
    }

    pub(crate) fn shader_data(&self) -> TerrainConfigUniform {
        TerrainConfigUniform {
            lod_count: self.lod_count,
            height: self.height,
            chunk_size: self.chunk_size,
        }
    }
}
