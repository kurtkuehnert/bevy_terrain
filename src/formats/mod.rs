//! The custom Terrain Data Format (TDF) that losslessly compresses the terrain data.
//!
//! It is based on the DTM and QOI format internally.

pub mod tc;
pub mod tdf;

use crate::formats::tdf::TDF;
use bevy::{
    asset::{AssetLoader, Error, LoadedAsset},
    prelude::*,
    render::render_resource::*,
};

struct TDFAssetLoader;

impl AssetLoader for TDFAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            let (descriptor, mut data) = TDF::decode_alloc(bytes, true).unwrap();

            // extend alpha channel
            if descriptor.channel_count == 3 && descriptor.pixel_size == 1 {
                data = data
                    .chunks_exact(3)
                    .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], u8::MAX])
                    .collect();
            };

            let image = Image {
                data,
                texture_descriptor: TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: descriptor.size,
                        height: descriptor.size,
                        ..default()
                    },
                    mip_level_count: descriptor.mip_level_count,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::R8Unorm,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                sampler_descriptor: Default::default(),
                texture_view_descriptor: None,
            };

            load_context.set_default_asset(LoadedAsset::new(image));

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tdf"]
    }
}

/// Plugin that registers the `TDFAssetLoader`.
pub struct TDFPlugin;

impl Plugin for TDFPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset_loader(TDFAssetLoader);
    }
    fn finish(&self, app: &mut App) {
     
    }
}
