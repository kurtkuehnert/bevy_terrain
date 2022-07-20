use crate::{terrain::Terrain, TerrainConfig, TerrainViewData};
use bevy::render::Extract;
use bevy::utils::Uuid;
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::*, renderer::RenderQueue},
    utils::HashMap,
};
use std::str::FromStr;

pub type TerrainViewComponents<C> = HashMap<(Entity, Entity), C>;

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

#[derive(Clone, Default, ShaderType)]
pub(crate) struct TerrainViewConfigUniform {
    height_under_viewer: f32,

    node_count: u32,

    tile_count: u32,
    refinement_count: u32,
    view_distance: f32,
    tile_scale: f32,

    morph_blend: f32,
    vertex_blend: f32,
    fragment_blend: f32,
}

#[derive(Clone, Component)]
pub struct TerrainViewConfig {
    pub(crate) quadtree_handle: Handle<Image>,

    pub height_under_viewer: f32,
    // quadtree
    pub load_distance: f32,
    pub node_count: u32,
    // tesselation
    pub tile_count: u32,
    pub refinement_count: u32,
    pub view_distance: f32,
    pub tile_scale: f32,
    pub morph_blend: f32,
    pub vertex_blend: f32,
    pub fragment_blend: f32,
}

impl TerrainViewConfig {
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

        let tile_count = 1000000;

        let view_distance = view_distance * config.chunk_size as f32; // same scale as load distance

        // let refinement_count = (config.terrain_size as f32 / tile_scale).log2().ceil() as u32;
        let refinement_count = 15;

        Self {
            quadtree_handle,
            height_under_viewer: 0.0,
            load_distance,
            node_count,
            tile_count,
            refinement_count,
            view_distance,
            tile_scale,
            morph_blend,
            vertex_blend,
            fragment_blend,
        }
    }

    pub(crate) fn change_tile_scale(&mut self, new: f32) {
        self.tile_scale = new;
        // self.refinement_count = (self.terrain_size as f32 / self.tile_scale).log2().ceil() as u32;
    }

    pub(crate) fn shader_data(&self) -> TerrainViewConfigUniform {
        TerrainViewConfigUniform {
            node_count: self.node_count,
            height_under_viewer: self.height_under_viewer,
            tile_count: self.tile_count,
            refinement_count: self.refinement_count,
            view_distance: self.view_distance,
            tile_scale: self.tile_scale,
            morph_blend: self.morph_blend,
            vertex_blend: self.vertex_blend,
            fragment_blend: self.fragment_blend,
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
