use bevy::prelude::*;
use bevy_inspector_egui::{Inspectable, RegisterInspectable};

#[derive(Component, Inspectable)]
pub struct TileHierarchyDescriptor {
    pub map_width: u16,
    pub map_height: u16,
    #[inspectable(min = 1, max = 181)]
    pub tile_size: u8, // sparse = 2 * tile_size, dense = 4 * tile_size
    pub wireframe: bool,
}

impl Default for TileHierarchyDescriptor {
    fn default() -> Self {
        Self {
            map_width: 30,
            map_height: 20,
            tile_size: 16,
            wireframe: true,
        }
    }
}

pub fn register_inspectable_types(app: &mut App) {
    app.register_inspectable::<TileHierarchyDescriptor>();
}
