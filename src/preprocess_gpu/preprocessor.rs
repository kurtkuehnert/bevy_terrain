use crate::{
    prelude::{NodeAtlas, TileConfig},
    preprocess_gpu::gpu_preprocessor::NodeMeta,
    terrain::Terrain,
    terrain_data::NodeCoordinate,
};
use bevy::{
    asset::LoadState,
    prelude::*,
    render::{render_resource::TextureFormat, texture::ImageSampler},
};

#[derive(Component)]
pub struct Preprocessor {
    pub(crate) tile_config: Option<TileConfig>,
    pub(crate) tile_handle: Option<Handle<Image>>,
    pub(crate) affected_nodes: Vec<NodeMeta>,
    pub(crate) is_ready: bool,
}

impl Preprocessor {
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

        for y in 0..2 {
            for x in 0..2 {
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
    mut terrain_query: Query<&mut Preprocessor, With<Terrain>>,
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
