//! Types for configuring terrain views.

use crate::{terrain::Terrain, TerrainConfig, TerrainViewData};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{extract_component::ExtractComponent, renderer::RenderQueue, Extract},
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
#[derive(Clone, Copy, Component)]
pub struct TerrainView;

impl ExtractComponent for TerrainView {
    type Query = Read<Self>;
    type Filter = ();

    #[inline]
    fn extract_component(_item: QueryItem<Self::Query>) -> Self {
        Self
    }
}

/// The configuration of a terrain view.
///
/// A terrain view describes the quality settings the corresponding terrain will be rendered with.
#[derive(Clone, Component)]
pub struct TerrainViewConfig {
    /// A handle to the quadtree texture.
    pub(crate) quadtree_handle: Handle<Image>,
    /// The current height under the viewer.
    pub height_under_viewer: f32,
    /// The distance (measured in node sizes) until which to request nodes to be loaded.
    pub load_distance: f32,
    /// The count of nodes in x and y direction per quadtree layer.
    pub node_count: u32,
    /// The size of the tile buffer.
    pub tile_count: u32,
    /// The amount of steps the tile list will be refined.
    pub refinement_count: u32,
    pub refinement_lod: u32,
    /// The distance (measured in node sizes) of each lod layer to the viewer.
    pub view_distance: f32,
    /// The size of the tiles.
    pub tile_scale: f32,
    /// The morph percentage of the mesh.
    pub morph_blend: f32,
    /// The blend percentage in the vertex shader.
    pub vertex_blend: f32,
    /// The blend percentage in the fragment shader.
    pub fragment_blend: f32,
}

impl TerrainViewConfig {
    /// Creates a new terrain view config for the terrain.
    pub fn new(
        config: &TerrainConfig,
        node_count: u32,
        load_distance: f32,
        view_distance: f32,
        tile_scale: f32,
        morph_blend: f32,
        vertex_blend: f32,
        fragment_blend: f32,
    ) -> Self {
        // Todo: fix this awful hack
        let quadtree_handle = HandleUntyped::weak_from_u64(
            Uuid::from_str("6ea26da6-6cf8-4ea2-9986-1d7bf6c17d6f").unwrap(),
            fastrand::u64(..),
        )
        .typed();

        let view_distance = view_distance * config.chunk_size as f32; // same scale as load distance

        // let refinement_count = (config.terrain_size as f32 / tile_scale).log2().ceil() as u32;
        // Todo: make these configurable ?
        let refinement_count = 20;
        let refinement_lod = 3;
        let tile_count = 1000000;
        let height_under_viewer = 0.0;

        Self {
            quadtree_handle,
            height_under_viewer,
            load_distance,
            node_count,
            tile_count,
            refinement_count,
            refinement_lod,
            view_distance,
            tile_scale,
            morph_blend,
            vertex_blend,
            fragment_blend,
        }
    }
}

pub(crate) fn extract_terrain_view_config(
    mut view_configs: ResMut<TerrainViewComponents<TerrainViewConfig>>,
    extracted_view_configs: Extract<Res<TerrainViewComponents<TerrainViewConfig>>>,
) {
    *view_configs = extracted_view_configs.clone();
}

pub(crate) fn queue_terrain_view_config(
    queue: Res<RenderQueue>,
    mut terrain_view_data: ResMut<TerrainViewComponents<TerrainViewData>>,
    view_configs: Res<TerrainViewComponents<TerrainViewConfig>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            let view_config = view_configs.get(&(terrain, view)).unwrap();
            let data = terrain_view_data.get_mut(&(terrain, view)).unwrap();
            data.update(&queue, view_config);
        }
    }
}
