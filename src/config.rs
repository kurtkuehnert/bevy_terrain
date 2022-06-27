use crate::attachment::{AtlasAttachmentConfig, AttachmentIndex};
use bevy::render::extract_component::ExtractComponent;
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::render_resource::*,
    utils::HashMap,
};

// Todo: fully reconsider the configuration

#[derive(Clone, Default, ShaderType)]
pub(crate) struct TerrainConfigUniform {
    lod_count: u32,
    height: f32,
    chunk_size: u32,

    node_count: u32,

    terrain_size: u32,
    patch_count: u32,
    refinement_count: u32,
    view_distance: f32,
    patch_scale: f32,
    patch_size: u32,
    vertices_per_row: u32,
    vertices_per_patch: u32,
}

#[derive(Clone, Component)]
pub struct TerrainConfig {
    // terrain
    pub lod_count: u32,
    pub height: f32,
    // quadtree
    pub load_distance: f32,
    pub node_count: u32,
    // tesselation
    pub terrain_size: u32,
    pub patch_count: u32,
    pub refinement_count: u32,
    pub view_distance: f32,
    pub patch_scale: f32,
    pub patch_size: u32,
    pub vertices_per_row: u32,
    pub vertices_per_patch: u32,
    // node atlas
    pub node_atlas_size: u16,
    pub chunk_size: u32,
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

    pub fn new(chunk_size: u32, lod_count: u32, height: f32, path: String) -> Self {
        let node_count = 8;
        let load_distance = 0.5 * node_count as f32;

        let terrain_size = 12000;
        let patch_count = 1000000;

        let patch_size = 16;
        let vertices_per_row = (patch_size + 2) << 1;
        let vertices_per_patch = vertices_per_row * patch_size;

        let view_distance = 3.0 * chunk_size as f32;

        let patch_scale = 4.0;
        let refinement_count = (terrain_size as f32 / (patch_scale * patch_size as f32))
            .log2()
            .ceil() as u32;

        let node_atlas_size = 2 * (lod_count * node_count * node_count) as u16;

        Self {
            lod_count,
            height,
            load_distance,
            node_count,
            patch_count,
            terrain_size,
            refinement_count,
            view_distance,
            patch_scale,
            patch_size,
            vertices_per_row,
            vertices_per_patch,
            node_atlas_size,
            chunk_size,
            path,
            attachments: default(),
        }
    }

    pub(crate) fn shader_data(&self) -> TerrainConfigUniform {
        TerrainConfigUniform {
            lod_count: self.lod_count,
            height: self.height,
            chunk_size: self.chunk_size,

            node_count: self.node_count,

            terrain_size: self.terrain_size,
            patch_count: self.patch_count,
            refinement_count: self.refinement_count,
            view_distance: self.view_distance,
            patch_size: self.patch_size,
            patch_scale: self.patch_scale,
            vertices_per_row: self.vertices_per_row,
            vertices_per_patch: self.vertices_per_patch,
        }
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
