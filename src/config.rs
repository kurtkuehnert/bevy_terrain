use crate::attachment::{AtlasAttachmentConfig, AttachmentIndex};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{render_component::ExtractComponent, render_resource::std140::AsStd140},
    utils::HashMap,
};
use itertools::{iproduct, Product};
use std::ops::Range;

// Todo: fully reconsider the configuration

pub type NodeId = u32;

pub struct NodePosition {
    pub lod: u32,
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Default, AsStd140)]
pub(crate) struct TerrainConfigUniform {
    lod_count: u32,
    patch_size: u32,
    chunk_size: u32,
    chunk_count: UVec2,
    area_size: u32,
    area_count: UVec2,
    terrain_size: UVec2,
    vertices_per_row: u32,
    scale: f32,
    height: f32,
    node_atlas_size: u32,
}

#[derive(Clone, Component)]
pub struct TerrainConfig {
    pub lod_count: u32,
    pub patch_size: u32,
    pub chunk_size: u32,
    pub chunk_count: UVec2,
    pub texture_size: u32,
    pub area_size: u32,
    pub area_count: UVec2,
    pub node_count: u32,
    pub terrain_size: UVec2,
    pub vertices_per_row: u32,
    pub scale: f32,
    pub height: f32,
    pub node_atlas_size: u16,
    pub cache_size: usize,
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
        let area_size = chunk_size * (1 << (lod_count - 1));
        let texture_size = chunk_size;
        let terrain_size = area_count * area_size;
        let chunk_count = area_count * (1 << (lod_count - 1));
        let vertices_per_row = (patch_size + 2) << 1;
        let node_count = area_count.x * area_count.y * ((1 << 2 * lod_count) - 1) / 3; // https://oeis.org/A002450

        Self {
            lod_count,
            patch_size,
            chunk_size,
            texture_size,
            chunk_count,
            area_size,
            area_count,
            node_count,
            terrain_size,
            vertices_per_row,
            scale,
            height,
            node_atlas_size,
            cache_size: 16,
            attachments: default(),
        }
    }

    pub(crate) fn as_std140(&self) -> Std140TerrainConfigUniform {
        TerrainConfigUniform {
            lod_count: self.lod_count,
            chunk_size: self.chunk_size,
            chunk_count: self.chunk_count,
            patch_size: self.patch_size,
            vertices_per_row: self.vertices_per_row,
            area_count: self.area_count,
            scale: self.scale,
            height: self.height,
            area_size: self.area_size,
            terrain_size: self.terrain_size,
            node_atlas_size: self.node_atlas_size as u32,
        }
        .as_std140()
    }

    #[inline]
    pub fn area_iter(&self) -> Product<Range<u32>, Range<u32>> {
        iproduct!(0..self.area_count.x, 0..self.area_count.y)
    }

    // Todo: consider storing these values as constants in arrays for each lod
    #[inline]
    pub fn node_count(&self, lod: u32) -> UVec2 {
        self.area_count * self.nodes_per_area(lod)
    }

    #[inline]
    pub fn nodes_per_area(&self, lod: u32) -> u32 {
        1 << (self.lod_count - lod - 1)
    }

    #[inline]
    pub fn node_size(&self, lod: u32) -> u32 {
        self.chunk_size * (1 << lod)
    }

    /// Calculates a unique identifier for the node at the specified position.
    /// These ids encode the position into 32 bits.
    pub fn node_id(lod: u32, x: u32, y: u32) -> NodeId {
        (lod & 0xF) << 28 | (x & 0x3FFF) << 14 | (y & 0x3FFF)
    }

    pub fn node_position(id: NodeId) -> NodePosition {
        NodePosition {
            lod: (id >> 28) & 0xF,
            x: (id >> 14) & 0x3FFF,
            y: id & 0x3FFF,
        }
    }
}

impl ExtractComponent for TerrainConfig {
    type Query = Read<TerrainConfig>;
    type Filter = Changed<TerrainConfig>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone() // Todo consider persisting the config in the render world
    }
}

mod tests {
    #[test]
    fn node_conversion() {
        let config = TerrainConfig::new(128, 3, UVec2::new(2, 2), 1.0, 0.0, 0);

        for (lod1, x1, y1) in iproduct!(0..3, 0..8, 0..8) {
            let id = TerrainConfig::node_id(lod1, x1, y1);
            let NodePosition {
                lod: lod2,
                x: x2,
                y: y2,
            } = TerrainConfig::node_position(id);

            assert_eq!(lod1, lod2);
            assert_eq!(x1, x2);
            assert_eq!(y1, y2);
        }
    }
}
