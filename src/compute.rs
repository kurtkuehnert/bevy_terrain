use crate::quadtree::NodeAtlas;
use crate::terrain::TerrainConfig;
use bevy::ecs::system::lifetimeless::{Read, SQuery, SResMut};
use bevy::ecs::system::{Command, SystemParamItem};
use bevy::reflect::TypeUuid;
use bevy::render::render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin};
use bevy::render::render_component::ExtractComponentPlugin;
use bevy::render::render_phase::AddRenderCommand;
use bevy::render::renderer::RenderQueue;
use bevy::{
    core_pipeline::node::MAIN_PASS_DEPENDENCIES,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        RenderApp, RenderStage,
    },
    window::WindowDescriptor,
};
use std::num::NonZeroU32;
use std::ops::Deref;

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "32a1cd80-cef4-4534-b0ec-bc3a3d0800a9"]
pub struct TerrainAsset {
    pub config: TerrainConfig,
}

pub struct GpuTerrain {
    pub(crate) quadtree_texture: Texture,
    pub(crate) draw_patch_buffer: Buffer,
}

impl RenderAsset for TerrainAsset {
    type ExtractedAsset = TerrainAsset;
    type PreparedAsset = GpuTerrain;
    type Param = (SResMut<RenderDevice>, SResMut<RenderQueue>);

    fn extract_asset(&self) -> Self::ExtractedAsset {
        TerrainAsset {
            config: self.config.clone(),
        }
    }

    fn prepare_asset(
        terrain: Self::ExtractedAsset,
        (device, queue): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        println!("init gpu terrain");

        let config = terrain.config;

        let texture_descriptor = TextureDescriptor {
            label: None,
            size: Extent3d {
                width: config.chunk_count.x,
                height: config.chunk_count.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: config.lod_count, // one mip level per lod
            sample_count: 1,
            dimension: TextureDimension::D2,
            // only r16 required, but storage textures only support r32 https://www.w3.org/TR/WGSL/#texel-formats
            format: TextureFormat::R32Uint,
            usage: TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING,
        };

        let quadtree_texture = device.create_texture(&texture_descriptor);

        // Todo: generate data all at once and only specify the offset

        for lod in 0..config.lod_count {
            let node_count = config.nodes_count(lod);

            let texture = ImageCopyTextureBase {
                texture: quadtree_texture.deref(),
                mip_level: lod,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All, // Todo: ?
            };

            let data_layout = ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::try_from(node_count.x * 4).unwrap()),
                rows_per_image: Some(NonZeroU32::try_from(node_count.y).unwrap()),
            };

            let size = Extent3d {
                width: node_count.x,
                height: node_count.y,
                depth_or_array_layers: 1,
            };

            let data: Vec<u32> = (0..node_count.x * node_count.y)
                .map(|_| NodeAtlas::INACTIVE_ID as u32)
                .collect();

            queue.write_texture(texture, bytemuck::cast_slice(&data), data_layout, size);
        }

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: 5 * 4,
            usage: BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        };

        let draw_patch_buffer = device.create_buffer(&buffer_descriptor);

        let data: [u32; 5] = [640, 3, 0, 0, 0];

        queue.write_buffer(&draw_patch_buffer, 0, bytemuck::cast_slice(&data));

        Ok(GpuTerrain {
            quadtree_texture,
            draw_patch_buffer,
        })
    }
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TerrainAsset>()
            .add_plugin(RenderAssetPlugin::<TerrainAsset>::default())
            .add_plugin(ExtractComponentPlugin::<Handle<TerrainAsset>>::default());

        let render_app = app.sub_app_mut(RenderApp);
    }
}
