//! Types for configuring terrain views.

use crate::{prelude::Quadtree, terrain::TerrainConfig};
use bevy::{prelude::*, render::extract_component::ExtractComponent, utils::HashMap};

/// Resource that stores components that are associated to a terrain entity and a view entity.
#[derive(Clone, Resource)]
pub struct TerrainViewComponents<C>(pub HashMap<(Entity, Entity), C>);

impl<C> TerrainViewComponents<C> {
    pub fn get(&self, k: &(Entity, Entity)) -> Option<&C> {
        self.0.get(k)
    }

    pub fn get_mut(&mut self, k: &(Entity, Entity)) -> Option<&mut C> {
        self.0.get_mut(k)
    }

    pub fn insert(&mut self, k: (Entity, Entity), v: C) {
        self.0.insert(k, v);
    }
}

impl<C> FromWorld for TerrainViewComponents<C> {
    fn from_world(_world: &mut World) -> Self {
        Self(default())
    }
}

/// A marker component used to identify a terrain view entity.
#[derive(Clone, Copy, Component, ExtractComponent)]
pub struct TerrainView;

/// The configuration of a terrain view.
///
/// A terrain view describes the quality settings the corresponding terrain will be rendered with.
#[derive(Clone, Component)]
pub struct TerrainViewConfig {
    /// The current height under the viewer.
    pub approximate_height: f32,
    /// The count of nodes in x and y direction per quadtree layer.
    pub quadtree_size: u32,
    /// The size of the tile buffer.
    pub tile_count: u32,
    /// The amount of steps the tile list will be refined.
    pub refinement_count: u32,
    /// The number of rows and columns of the tile grid.
    pub grid_size: u32,
    // Todo: set scale for these appropriately
    pub load_distance: f32,
    pub morph_distance: f32,
    pub blend_distance: f32,
    /// The morph percentage of the mesh.
    pub morph_range: f32,
    /// The blend percentage in the vertex and fragment shader.
    pub blend_range: f32,
    pub precision_threshold_distance: f32,
}

impl Default for TerrainViewConfig {
    fn default() -> Self {
        Self {
            approximate_height: 0.0,
            quadtree_size: 4,
            tile_count: 1000000,
            refinement_count: 30,
            grid_size: 32,
            load_distance: 3.0,
            morph_distance: 8.0,
            blend_distance: 1.0,
            morph_range: 0.2,
            blend_range: 0.2,
            precision_threshold_distance: 5000.0,
        }
    }
}

pub fn initialize_terrain_view(
    terrain: Entity,
    view: Entity,
    config: &TerrainConfig,
    view_config: TerrainViewConfig,
    quadtrees: &mut TerrainViewComponents<Quadtree>,
    view_configs: &mut TerrainViewComponents<TerrainViewConfig>,
) {
    let quadtree = Quadtree::from_configs(config, &view_config);
    view_configs.insert((terrain, view), view_config);
    quadtrees.insert((terrain, view), quadtree);
}
