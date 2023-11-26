use crate::{
    terrain::{Terrain, TerrainConfig},
    terrain_data::node_atlas::NodeAtlas,
};
use bevy::{prelude::*, render::view::NoFrustumCulling};

/// The components of a terrain.
///
/// Does not include loader(s) and a material.
#[derive(Bundle)]
pub struct TerrainBundle {
    pub terrain: Terrain,
    pub node_atlas: NodeAtlas,
    pub config: TerrainConfig,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility_bundle: VisibilityBundle,
    pub no_frustum_culling: NoFrustumCulling,
}

impl TerrainBundle {
    /// Creates a new terrain bundle from the config.
    pub fn new(config: TerrainConfig, translation: Vec3, scale: f32) -> Self {
        Self {
            terrain: Terrain,
            node_atlas: NodeAtlas::from_config(&config),
            config,
            transform: Transform {
                translation,
                scale: Vec3::splat(scale),
                ..default()
            },
            global_transform: default(),
            visibility_bundle: default(),
            no_frustum_culling: NoFrustumCulling,
        }
    }
}
