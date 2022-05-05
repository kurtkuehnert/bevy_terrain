use bevy::prelude::*;

/// Marks a camera as the viewer of the terrain.
/// The view distance is a multiplier, which increases the amount of loaded nodes.
#[derive(Component)]
pub struct ViewDistance {
    // #[inspectable(min = 1.0)]
    pub view_distance: f32,
}

impl Default for ViewDistance {
    fn default() -> Self {
        Self { view_distance: 8.0 }
    }
}

#[derive(Clone, Copy)]
pub struct Viewer {
    pub(crate) position: Vec2,
    pub(crate) view_distance: f32,
}
