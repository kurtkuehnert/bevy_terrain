use bevy::prelude::*;

#[derive(Debug)]
pub struct TerrainConfig {
    pub lod_count: u32,
    pub patch_count: u32,
    pub patch_size: u32,
    pub chunk_size: u32,
    pub area_size: u32,
    pub area_count: UVec2,
    pub map_size: UVec2,
}

impl TerrainConfig {
    pub fn new(chunk_size: u32, lod_count: u32, area_count: UVec2) -> Self {
        let patch_count = 8;
        let patch_size = chunk_size / patch_count;
        let area_size = chunk_size * (1 << lod_count - 1);
        let map_size = area_count * area_size;

        Self {
            lod_count,
            patch_count,
            patch_size,
            chunk_size,
            area_size,
            area_count,
            map_size,
        }
    }

    #[inline]
    pub fn node_count(&self, lod: u32) -> u32 {
        1 << self.lod_count - lod - 1
    }

    #[inline]
    pub fn node_size(&self, lod: u32) -> u32 {
        self.chunk_size * 1 << lod
    }

    #[inline]
    fn nodes_with_lod(&self, lod: u32) -> u32 {
        self.area_count.x * self.area_count.y * self.node_count(lod).pow(2)
    }

    /// Calculates a unique index for the node at the specified position.
    /// These indices are tightly packed (no gaps).
    pub fn node_id(&self, lod: u32, x: u32, y: u32) -> u16 {
        let width = self.area_count.x * self.node_count(lod);
        let offset: u32 = (0..lod).map(|i| self.nodes_with_lod(i)).sum();

        (offset + x + y * width) as u16
    }

    pub fn node_position(&self, id: u16) -> (u32, u32, u32) {
        let id = id as u32;
        let mut count = 0;

        for lod in 0..self.lod_count {
            let new_count = count + self.nodes_with_lod(lod);

            if new_count > id {
                let width = self.area_count.x * self.node_count(lod);
                let pos = id - count;

                return (lod, pos % width, pos / width);
            }

            count = new_count;
        }

        panic!("Invalid id: {id}!")
    }
}

mod tests {
    use crate::terrain::TerrainConfig;
    use bevy::prelude::*;

    #[test]
    fn node_conversion() {
        let config = TerrainConfig::new(128, 3, UVec2::new(2, 2));

        for i in 0..84 {
            let (lod, x, y) = config.node_position(i);
            let id = config.node_id(lod, x, y);

            assert_eq!(i, id);
        }
    }
}
