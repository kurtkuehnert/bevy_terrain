use crate::material::TerrainMaterial;
use bevy::prelude::*;

#[derive(Default, Bundle)]
pub struct PieceBundle {
    pub mesh: Handle<Mesh>,
    pub material: Handle<TerrainMaterial>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}
