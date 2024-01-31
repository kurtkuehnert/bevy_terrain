use bevy::render::render_asset::RenderAssetUsages;
use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::{ImageLoaderSettings, TextureError},
    },
};
use bytemuck::cast_slice;
use std::io::Cursor;
use tiff::decoder::{Decoder, DecodingResult};

#[derive(Default)]
pub struct TiffLoader;
impl AssetLoader for TiffLoader {
    type Asset = Image;
    type Settings = ImageLoaderSettings;
    type Error = TextureError;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Image, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await.unwrap();

            let mut decoder = Decoder::new(Cursor::new(bytes)).unwrap();

            let (width, height) = decoder.dimensions().unwrap();

            let data = match decoder.read_image().unwrap() {
                DecodingResult::U8(data) => cast_slice(&data).to_vec(),
                DecodingResult::U16(data) => cast_slice(&data).to_vec(),
                DecodingResult::U32(data) => cast_slice(&data).to_vec(),
                DecodingResult::U64(data) => cast_slice(&data).to_vec(),
                DecodingResult::F32(data) => cast_slice(&data).to_vec(),
                DecodingResult::F64(data) => cast_slice(&data).to_vec(),
                DecodingResult::I8(data) => cast_slice(&data).to_vec(),
                DecodingResult::I16(data) => cast_slice(&data).to_vec(),
                DecodingResult::I32(data) => cast_slice(&data).to_vec(),
                DecodingResult::I64(data) => cast_slice(&data).to_vec(),
            };

            let mut image = Image::new(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                TextureFormat::R16Unorm,
                RenderAssetUsages::default(),
            );

            settings.apply_to(&mut image.texture_descriptor);

            Ok(image)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tif", "tiff"]
    }
}
