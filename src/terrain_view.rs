use crate::{terrain::Terrain, TerrainViewData};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Read},
    prelude::*,
    render::{
        extract_component::ExtractComponent, render_resource::*, renderer::RenderQueue, RenderWorld,
    },
    utils::HashMap,
};

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

    terrain_size: u32,
    patch_count: u32,
    refinement_count: u32,
    view_distance: f32,
    patch_scale: f32,
}

#[derive(Clone, Component)]
pub struct TerrainViewConfig {
    pub height_under_viewer: f32,
    // quadtree
    pub load_distance: f32,
    pub node_count: u32,
    // tesselation
    pub terrain_size: u32,
    pub patch_count: u32,
    pub refinement_count: u32,
    pub view_distance: f32,
    pub patch_scale: f32,
}

impl TerrainViewConfig {
    pub fn new(
        terrain_size: u32,
        view_distance: f32,
        patch_scale: f32,
        load_distance: f32,
    ) -> Self {
        let node_count = 12;
        let load_distance = load_distance * node_count as f32;

        let patch_count = 1000000;

        let view_distance = view_distance * 128.0;

        let refinement_count = (terrain_size as f32 / patch_scale).log2().ceil() as u32;

        Self {
            height_under_viewer: 0.0,
            load_distance,
            node_count,
            patch_count,
            terrain_size,
            refinement_count,
            view_distance,
            patch_scale,
        }
    }

    pub(crate) fn change_patch_scale(&mut self, new: f32) {
        self.patch_scale = new;
        self.refinement_count = (self.terrain_size as f32 / self.patch_scale).log2().ceil() as u32;
    }

    pub(crate) fn shader_data(&self) -> TerrainViewConfigUniform {
        TerrainViewConfigUniform {
            node_count: self.node_count,
            height_under_viewer: self.height_under_viewer,
            terrain_size: self.terrain_size,
            patch_count: self.patch_count,
            refinement_count: self.refinement_count,
            view_distance: self.view_distance,
            patch_scale: self.patch_scale,
        }
    }
}

pub(crate) fn extract_terrain_view_config(
    mut render_world: ResMut<RenderWorld>,
    view_configs: Res<TerrainViewComponents<TerrainViewConfig>>,
) {
    render_world.insert_resource(view_configs.clone());
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
