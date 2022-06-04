use crate::attachment::{AtlasAttachmentConfig, AttachmentIndex};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{render_component::ExtractComponent, render_resource::*},
    utils::HashMap,
};

// Todo: fully reconsider the configuration

#[derive(Clone, Default, ShaderType)]
pub(crate) struct TerrainConfigUniform {
    lod_count: u32,
    chunk_size: u32,
    patch_size: u32,
    node_count: u32,
    vertices_per_row: u32,
    vertices_per_patch: u32,
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
    pub area_count: UVec2,
    pub node_count: u32,
    pub view_distance: f32,
    pub load_distance: f32,
    pub vertices_per_row: u32,
    pub vertices_per_patch: u32,
    pub scale: f32,
    pub height: f32,
    pub node_atlas_size: u16,
    pub path: String,
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
        path: String,
    ) -> Self {
        let chunk_count = area_count * (1 << (lod_count - 1));

        let patch_size = 16;
        let view_distance = 1.5 * chunk_size as f32; // half of the view radius

        // let patch_size = 2;
        // let view_distance = 0.1 * chunk_size as f32; // half of the view radius

        let vertices_per_row = (patch_size + 2) << 1;
        let vertices_per_patch = vertices_per_row * patch_size;

        let node_count = 8;
        let load_distance = 0.5 * node_count as f32;
        let node_atlas_size = (lod_count * node_count * node_count) as u16;

        Self {
            lod_count,
            patch_size,
            chunk_size,
            chunk_count,
            area_count,
            node_count,
            view_distance,
            load_distance,
            vertices_per_row,
            vertices_per_patch,
            scale,
            height,
            node_atlas_size,
            path,
            attachments: default(),
        }
    }

    pub(crate) fn shader_data(&self) -> TerrainConfigUniform {
        TerrainConfigUniform {
            lod_count: self.lod_count,
            chunk_size: self.chunk_size,
            patch_size: self.patch_size,
            node_count: self.node_count,
            vertices_per_row: self.vertices_per_row,
            vertices_per_patch: self.vertices_per_patch,
            view_distance: self.view_distance,
            scale: self.scale,
            height: self.height,
        }
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
