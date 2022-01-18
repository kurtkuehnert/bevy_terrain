use crate::quadtree::{Quadtree, Viewer};
use bevy::prelude::*;
use bevy_inspector_egui::{Inspectable, RegisterInspectable};

#[derive(Component, Inspectable)]
pub struct QuadtreeDescriptor {
    #[inspectable(min = 2)]
    pub node_size: u8,
    #[inspectable(min = 1, max = 16)]
    pub lod_count: u8,
    pub wireframe: bool,
}

impl Default for QuadtreeDescriptor {
    fn default() -> Self {
        Self {
            node_size: 32,
            lod_count: 6,
            wireframe: true,
        }
    }
}

pub fn register_inspectable_types(app: &mut App) {
    app.register_inspectable::<QuadtreeDescriptor>();
    app.register_inspectable::<Viewer>();
    app.register_inspectable::<Quadtree>();
}
