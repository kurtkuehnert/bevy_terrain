use crate::terrain_data::{
    attachment::{AttachmentData, AttachmentFormat},
    tile_atlas::{AtlasTileAttachment, AtlasTileAttachmentWithData, TileAtlas},
};
use bevy::{
    asset::{AssetServer, Assets, Handle},
    image::Image,
    prelude::*,
};
use slab::Slab;

struct LoadingTile {
    handle: Handle<Image>,
    tile: AtlasTileAttachment,
    texture_size: u32,
    format: AttachmentFormat,
    mip_level_count: u32,
}

#[derive(Component)]
pub struct DefaultLoader {
    loading_tiles: Slab<LoadingTile>,
}

impl Default for DefaultLoader {
    fn default() -> Self {
        Self {
            loading_tiles: Slab::with_capacity(4),
        }
    }
}

impl DefaultLoader {
    fn to_load_next(&self, tiles: &mut Vec<AtlasTileAttachment>) -> Option<AtlasTileAttachment> {
        // Todo: tile prioritization goes here
        tiles.pop()
    }

    fn finish_loading(
        &mut self,
        atlas: &mut TileAtlas,
        asset_server: &mut AssetServer,
        images: &mut Assets<Image>,
    ) {
        self.loading_tiles.retain(|_, tile| {
            if asset_server.is_loaded(tile.handle.id()) {
                // Todo: generating mip maps takes time -> this should run asynchronously

                let image = images.get(tile.handle.id()).unwrap();

                let mut data = AttachmentData::from_bytes(&image.data, tile.format);
                data.generate_mipmaps(tile.texture_size, tile.mip_level_count);

                let tile = AtlasTileAttachmentWithData {
                    tile: tile.tile.clone(),
                    data,
                };

                atlas.tile_loaded(tile);

                false
            } else {
                true
            }
        });
    }

    fn start_loading(&mut self, atlas: &mut TileAtlas, asset_server: &mut AssetServer) {
        while self.loading_tiles.len() < self.loading_tiles.capacity() {
            if let Some(tile) = self.to_load_next(&mut atlas.to_load) {
                let attachment = atlas.attachments.get(&tile.label).unwrap();

                let path = tile
                    .coordinate
                    .path(&attachment.path.join(String::from(&tile.label)));

                self.loading_tiles.insert(LoadingTile {
                    handle: asset_server.load(path),
                    tile,
                    texture_size: attachment.texture_size,
                    format: attachment.format,
                    mip_level_count: attachment.mip_level_count,
                });
            } else {
                break;
            }
        }
    }
}

pub fn finish_loading(
    mut terrains: Query<(&mut TileAtlas, &mut DefaultLoader)>,
    mut asset_server: ResMut<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    for (mut tile_atlas, mut loader) in &mut terrains {
        loader.finish_loading(&mut tile_atlas, &mut asset_server, &mut images);
    }
}

pub fn start_loading(
    mut terrains: Query<(&mut TileAtlas, &mut DefaultLoader)>,
    mut asset_server: ResMut<AssetServer>,
) {
    for (mut tile_atlas, mut loader) in &mut terrains {
        loader.start_loading(&mut tile_atlas, &mut asset_server);
    }
}
