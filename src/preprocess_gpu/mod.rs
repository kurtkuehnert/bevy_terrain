use crate::prelude::{NodeAtlas, TileConfig};
use crate::preprocess_gpu::preprocess_data::{
    extract_terrain_preprocessor, prepare_terrain_preprocessor, NodeMeta, PreprocessData,
};
use crate::preprocess_gpu::preprocess_pipeline::{
    TerrainPreprocessNode, TerrainPreprocessPipelines,
};
use crate::terrain::{Terrain, TerrainComponents};
use crate::terrain_data::NodeCoordinate;
use bevy::asset::LoadState;
use bevy::prelude::*;
use bevy::render::main_graph::node::CAMERA_DRIVER;
use bevy::render::render_graph::RenderGraph;
use bevy::render::render_resource::{SpecializedComputePipelines, TextureFormat};
use bevy::render::texture::ImageSampler;
use bevy::render::{Render, RenderApp, RenderSet};

pub mod preprocess_data;
pub mod preprocess_pipeline;
pub mod shaders;

#[derive(Component)]
pub struct NewPreprocessor {
    tile_config: Option<TileConfig>,
    tile_handle: Option<Handle<Image>>,
    affected_nodes: Vec<NodeMeta>,
    is_ready: bool,
}

impl NewPreprocessor {
    pub fn new() -> Self {
        Self {
            tile_config: None,
            tile_handle: None,
            affected_nodes: vec![],
            is_ready: false,
        }
    }

    pub fn preprocess_tile(
        &mut self,
        tile_config: TileConfig,
        asset_server: &AssetServer,
        node_atlas: &mut NodeAtlas,
    ) {
        self.tile_config = Some(tile_config.clone());
        self.tile_handle = Some(asset_server.load(tile_config.path));

        for y in 0..8 {
            for x in 0..8 {
                let node_coordinate = NodeCoordinate {
                    side: 0,
                    lod: 0,
                    x,
                    y,
                };

                let atlas_index = node_atlas.allocate(node_coordinate.clone());

                self.affected_nodes.push(NodeMeta {
                    atlas_index: atlas_index as u32,
                    node_coordinate,
                });
            }
        }
    }
}

pub(crate) fn preprocessor_is_ready(
    asset_server: Res<AssetServer>,
    mut terrain_query: Query<&mut NewPreprocessor, With<Terrain>>,
    mut images: ResMut<Assets<Image>>,
) {
    for mut preprocessor in terrain_query.iter_mut() {
        if let Some(handle) = &preprocessor.tile_handle {
            if preprocessor.is_ready {
                preprocessor.is_ready = false;
                preprocessor.tile_handle = None;
            } else if asset_server.load_state(handle) == LoadState::Loaded {
                let image = images.get_mut(handle).unwrap();
                image.texture_descriptor.format = TextureFormat::R16Unorm;
                image.sampler = ImageSampler::linear();

                preprocessor.is_ready = true;
            }
        }
    }
}

pub struct TerrainPreprocessPlugin;

impl Plugin for TerrainPreprocessPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, preprocessor_is_ready);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<TerrainComponents<PreprocessData>>()
                .add_systems(ExtractSchedule, extract_terrain_preprocessor)
                .add_systems(
                    Render,
                    prepare_terrain_preprocessor.in_set(RenderSet::PrepareAssets),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        let render_app = app
            .sub_app_mut(RenderApp)
            .init_resource::<SpecializedComputePipelines<TerrainPreprocessPipelines>>()
            .init_resource::<TerrainPreprocessPipelines>();

        let preprocess_node = TerrainPreprocessNode::from_world(&mut render_app.world);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("terrain_preprocess", preprocess_node);
        render_graph.add_node_edge("terrain_preprocess", CAMERA_DRIVER);
    }
}
