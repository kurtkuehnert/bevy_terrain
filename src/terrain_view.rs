//! Types for configuring terrain views.

use bevy::{
    prelude::*,
    render::extract_component::ExtractComponent,
    utils::{HashMap, Uuid},
};
use std::str::FromStr;

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
    /// A handle to the quadtree texture.
    pub quadtree_handle: Handle<Image>,
    /// The current height under the viewer.
    pub height_under_viewer: f32,
    /// The count of nodes in x and y direction per quadtree layer.
    pub node_count: u32,
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
}

impl Default for TerrainViewConfig {
    fn default() -> Self {
        Self {
            quadtree_handle: HandleUntyped::weak_from_u64(
                Uuid::from_str("6ea26da6-6cf8-4ea2-9986-1d7bf6c17d6f").unwrap(),
                fastrand::u64(..),
            )
            .typed(), // Todo: fix this awful hack
            height_under_viewer: 0.0,
            node_count: 10,
            tile_count: 1000000,
            refinement_count: 20,
            grid_size: 8,
            load_distance: 5.0,
            morph_distance: 4.0,
            blend_distance: 4.0,
            morph_range: 0.2,
            blend_range: 0.2,
        }
    }
}
