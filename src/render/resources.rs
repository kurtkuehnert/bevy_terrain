use bevy::{
    prelude::*,
    render::{
        render_resource::{std140::Std140, *},
        renderer::RenderDevice,
    },
};

use crate::render::InitTerrain;
use crate::{config::TerrainConfig, render::layouts::*};

pub enum NodeAttachment {}

#[derive(Component)]
pub struct TerrainResources {
    pub(crate) indirect_buffer: Option<Buffer>,
    pub(crate) parameter_buffer: Buffer,
    pub(crate) config_buffer: Buffer,
    pub(crate) temp_node_buffers: [Buffer; 2],
    pub(crate) final_node_buffer: Buffer,
    pub(crate) patch_buffer: Buffer,
    pub(crate) lod_map_view: TextureView,
    pub(crate) atlas_map_view: TextureView,
}

impl TerrainResources {
    pub(crate) fn new(config: &TerrainConfig, device: &RenderDevice) -> Self {
        let indirect_buffer = Some(Self::create_indirect_buffer(device));
        let parameter_buffer = Self::create_parameter_buffer(device);
        let config_buffer = Self::create_config_buffer(config, device);
        let (temp_node_buffers, final_node_buffer) = Self::create_node_buffers(config, device);
        let patch_buffer = Self::create_patch_buffer(config, device);
        let (lod_map_view, atlas_map_view) = Self::create_chunk_maps(config, device);

        Self {
            indirect_buffer,
            parameter_buffer,
            config_buffer,
            temp_node_buffers,
            final_node_buffer,
            patch_buffer,
            lod_map_view,
            atlas_map_view,
        }
    }

    fn create_indirect_buffer(device: &RenderDevice) -> Buffer {
        device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            contents: &[0; INDIRECT_BUFFER_SIZE as usize],
        })
    }

    fn create_config_buffer(config: &TerrainConfig, device: &RenderDevice) -> Buffer {
        device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: config.as_std140().as_bytes(),
        })
    }

    fn create_parameter_buffer(device: &RenderDevice) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: None,
            size: PARAMETER_BUFFER_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    fn create_node_buffers(config: &TerrainConfig, device: &RenderDevice) -> ([Buffer; 2], Buffer) {
        let max_node_count = config.chunk_count.x * config.chunk_count.y;

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: NODE_SIZE * max_node_count as BufferAddress,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        (
            [
                device.create_buffer(&buffer_descriptor),
                device.create_buffer(&buffer_descriptor),
            ],
            device.create_buffer(&buffer_descriptor),
        )
    }

    fn create_patch_buffer(config: &TerrainConfig, device: &RenderDevice) -> Buffer {
        let max_patch_count =
            config.chunk_count.x * config.chunk_count.y * TerrainConfig::PATCHES_PER_NODE;

        let buffer_descriptor = BufferDescriptor {
            label: None,
            size: PATCH_SIZE * max_patch_count as BufferAddress,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        device.create_buffer(&buffer_descriptor)
    }

    fn create_chunk_maps(
        config: &TerrainConfig,
        device: &RenderDevice,
    ) -> (TextureView, TextureView) {
        let chunk_count = config.chunk_count;

        let lod_map = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: chunk_count.x,
                height: chunk_count.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Uint,
            usage: TextureUsages::COPY_DST
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING,
        });

        let atlas_map = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: chunk_count.x,
                height: chunk_count.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Uint,
            usage: TextureUsages::COPY_DST
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING,
        });

        let lod_map_view = lod_map.create_view(&TextureViewDescriptor {
            label: None,
            format: Some(TextureFormat::R8Uint),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let atlas_map_view = atlas_map.create_view(&TextureViewDescriptor {
            label: None,
            format: Some(TextureFormat::R16Uint),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        (lod_map_view, atlas_map_view)
    }
}

/// Runs in prepare.
pub(crate) fn init_terrain_resources(
    mut commands: Commands,
    device: Res<RenderDevice>,
    terrain_query: Query<(Entity, &TerrainConfig), With<InitTerrain>>,
) {
    for (entity, config) in terrain_query.iter() {
        info!("initializing terrain resources");

        commands
            .get_or_spawn(entity)
            .insert(TerrainResources::new(config, &device));
    }
}
