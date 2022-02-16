use bevy::{prelude::*, render::render_resource::std140::AsStd140};
use itertools::{iproduct, Product};
use std::ops::Range;

#[derive(Clone, Default, AsStd140)]
pub(crate) struct TerrainConfigUniform {
    lod_count: u32,
    chunk_size: u32,
    area_count: UVec2,
    scale: f32,
    height: f32,
}

#[derive(Clone, Debug, Component)]
pub struct TerrainConfig {
    pub lod_count: u32,
    pub patch_size: u32,
    pub patch_count: u32,
    pub chunk_size: u32,
    pub chunk_count: UVec2,
    pub texture_size: u32,
    pub area_size: u32,
    pub area_count: UVec2,
    pub terrain_size: UVec2,
    pub scale: f32,
    pub height: f32,
    pub node_atlas_size: u16,
}

impl TerrainConfig {
    pub fn new(
        chunk_size: u32,
        lod_count: u32,
        area_count: UVec2,
        scale: f32,
        height: f32,
        node_atlas_size: u16,
    ) -> Self {
        let patch_count = 8;
        let patch_size = chunk_size / patch_count;
        let area_size = chunk_size * (1 << (lod_count - 1));
        let texture_size = chunk_size + 1;
        let terrain_size = area_count * area_size;
        let chunk_count = area_count * (1 << (lod_count - 1));

        Self {
            lod_count,
            patch_size,
            patch_count,
            chunk_size,
            texture_size,
            chunk_count,
            area_size,
            area_count,
            terrain_size,
            scale,
            height,
            node_atlas_size,
        }
    }

    pub(crate) fn as_std140(&self) -> Std140TerrainConfigUniform {
        TerrainConfigUniform {
            lod_count: self.lod_count,
            chunk_size: self.chunk_size,
            area_count: self.area_count,
            scale: self.scale,
            height: self.height,
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
    pub fn node_id(lod: u32, x: u32, y: u32) -> u32 {
        (lod & 0xF) << 28 | (x & 0x3FFF) << 14 | (y & 0x3FFF)
    }

    pub fn node_position(id: u32) -> (u32, u32, u32) {
        ((id >> 28) & 0xF, (id >> 14) & 0x3FFF, id & 0x3FFF)
    }
}

mod tests {
    #[test]
    fn node_conversion() {
        let config = TerrainConfig::new(128, 3, UVec2::new(2, 2));

        for (lod1, x1, y1) in iproduct!(0..3, 0..8, 0..8) {
            let id = TerrainConfig::node_id(lod1, x1, y1);
            let (lod2, x2, y2) = TerrainConfig::node_position(id);

            assert_eq!(lod1, lod2);
            assert_eq!(x1, x2);
            assert_eq!(y1, y2);
        }
    }
}
