//! The custom Terrain Data Format (TDF) that losslessly compresses the terrain data.
//!
//! It is based on the DTM and QOI format internally.

pub mod tc;
pub mod tdf;
pub mod tiff;

use crate::formats::tdf::TDF;
use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    render::{render_asset::RenderAssetPersistencePolicy, render_resource::*},
    utils::BoxedFuture,
};

#[derive(Default)]
struct TDFAssetLoader;

impl AssetLoader for TDFAssetLoader {
    type Asset = Image;
    type Settings = ();
    type Error = std::io::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let (descriptor, mut data) = TDF::decode_alloc(&bytes, true).unwrap();

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
                sampler: Default::default(),
                texture_view_descriptor: None,
                cpu_persistent_access: RenderAssetPersistencePolicy::Keep,
            };

            Ok(image)
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
        app.init_asset_loader::<TDFAssetLoader>();
    }
}
