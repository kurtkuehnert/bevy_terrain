#[derive(Debug)]
pub struct TerrainConfig {
    pub lod_count: u32,
    pub width: u32,
    pub height: u32,
    pub patch_count: u32,
    pub patch_size: u32,
    pub chunk_size: u32,
    pub area_size: u32,
    pub area_count_x: u32,
    pub area_count_y: u32,
}

impl TerrainConfig {
    pub fn new(chunk_size: u32, lod_count: u32, area_count_x: u32, area_count_y: u32) -> Self {
        let patch_count = 8;
        let patch_size = chunk_size / patch_count;
        let area_size = chunk_size * (1 << lod_count - 1);
        let width = area_count_x * area_size;
        let height = area_count_y * area_size;

        Self {
            lod_count,
            width,
            height,
            patch_count,
            patch_size,
            chunk_size,
            area_size,
            area_count_x,
            area_count_y,
        }
    }

    #[inline]
    pub fn node_count(&self, lod: u32) -> u32 {
        1 << self.lod_count - lod - 1
    }

    /// Calculates a unique index for the node at the specified position.
    /// These indices are tightly packed (no gaps).
    pub fn calculate_node_id(&self, lod: u32, x: u32, y: u32) -> u16 {
        let width = self.area_count_x * self.node_count(lod);
        let offset: u32 = (0..lod)
            .map(|i| self.area_count_x * self.area_count_y * self.node_count(i).pow(2))
            .sum();

        (offset + x + y * width) as u16
    }
}
