use crate::render::terrain_pipeline::TerrainPipeline;
use crate::terrain::{TerrainConfig, TerrainConfigUniform};
use bevy::render::render_resource::std140::AsStd140;
use bevy::render::render_resource::std140::Std140;
use bevy::{
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
    },
};
use std::mem;

pub struct GpuTerrainData {
    pub(crate) bind_group: BindGroup,
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "32a1cd80-cef4-4534-b0ec-bc3a3d0800a9"]
pub struct TerrainData {
    pub(crate) config: TerrainConfig,
    pub height_texture: Handle<Image>, // Todo: replace in favor of the node atlas
}

impl RenderAsset for TerrainData {
    type ExtractedAsset = TerrainData;
    type PreparedAsset = GpuTerrainData;
    type Param = (
        SRes<RenderDevice>,
        SRes<TerrainPipeline>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        terrain_data: Self::ExtractedAsset,
        (device, pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        println!("init gpu terrain");

        let height_texture_view = match gpu_images.get(&terrain_data.height_texture) {
            Some(gpu_image) => &gpu_image.texture_view,
            None => return Err(PrepareAssetError::RetryNextUpdate(terrain_data)),
        };

        let terrain_config_uniform: TerrainConfigUniform = terrain_data.config.into();
        let terrain_config_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: terrain_config_uniform.as_std140().as_bytes(),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: terrain_config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(height_texture_view),
                },
            ],
            label: None,
            layout: &pipeline.terrain_data_layout,
        });

        Ok(GpuTerrainData { bind_group })
    }
}
