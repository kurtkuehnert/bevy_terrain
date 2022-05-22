use crate::attachment::{AtlasAttachmentConfig, AttachmentIndex};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{render_component::ExtractComponent, render_resource::std140::AsStd140},
    utils::HashMap,
};

// Todo: fully reconsider the configuration

#[derive(Clone, Default, AsStd140)]
pub(crate) struct TerrainConfigUniform {
    lod_count: u32,
    chunk_size: u32,
    patch_size: u32,
    node_count: u32,
    vertices_per_row: u32,
    view_distance: f32,
    scale: f32,
    height: f32,
}

#[derive(Clone, Component)]
pub struct TerrainConfig {
    pub lod_count: u32,
    pub patch_size: u32,
    pub chunk_size: u32,
    pub chunk_count: UVec2,
    pub texture_size: u32,
    pub area_count: UVec2,
    pub node_count: u32,
    pub view_distance: f32,
    pub vertices_per_row: u32,
    pub scale: f32,
    pub height: f32,
    pub node_atlas_size: u16,
    pub attachments: HashMap<AttachmentIndex, AtlasAttachmentConfig>,
}

impl TerrainConfig {
    pub const PATCH_COUNT: u32 = 8;
    pub const PATCHES_PER_NODE: u32 = 64;

    pub fn add_attachment(
        &mut self,
        attachment_index: AttachmentIndex,
        attachment_config: AtlasAttachmentConfig,
    ) {
        self.attachments.insert(attachment_index, attachment_config);
    }

    pub fn new(
        chunk_size: u32,
        lod_count: u32,
        area_count: UVec2,
        scale: f32,
        height: f32,
        node_atlas_size: u16,
    ) -> Self {
        let patch_size = chunk_size / Self::PATCH_COUNT;
        let texture_size = chunk_size;
        let chunk_count = area_count * (1 << (lod_count - 1));
        let vertices_per_row = (patch_size + 2) << 1;

        let view_distance = 6.0 * (patch_size * 2) as f32;
        let node_count = 8;

        Self {
            lod_count,
            patch_size,
            chunk_size,
            texture_size,
            chunk_count,
            area_count,
            node_count,
            view_distance,
            vertices_per_row,
            scale,
            height,
            node_atlas_size,
            attachments: default(),
        }
    }

    pub(crate) fn as_std140(&self) -> Std140TerrainConfigUniform {
        TerrainConfigUniform {
            lod_count: self.lod_count,
            chunk_size: self.chunk_size,
            patch_size: self.patch_size,
            node_count: self.node_count,
            vertices_per_row: self.vertices_per_row,
            view_distance: self.view_distance,
            scale: self.scale,
            height: self.height,
        }
        .as_std140()
    }

    #[inline]
    pub(crate) fn node_size(&self, lod: u32) -> u32 {
        self.chunk_size * (1 << lod)
    }

    #[inline]
    pub fn nodes_per_area(&self, lod: u32) -> u32 {
        1 << (self.lod_count - lod - 1)
    }
}

impl ExtractComponent for TerrainConfig {
    type Query = Read<TerrainConfig>;
    type Filter = Changed<TerrainConfig>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}
