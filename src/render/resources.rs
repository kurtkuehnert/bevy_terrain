use crate::{
    config::TerrainConfig,
    render::{layouts::*, InitTerrain},
};
use bevy::{
    prelude::*,
    render::{
        render_resource::{std140::Std140, *},
        renderer::RenderDevice,
    },
};

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
    pub(crate) fn new(device: &RenderDevice, config: &TerrainConfig) -> Self {
        let indirect_buffer = Some(Self::create_indirect_buffer(device));
        let parameter_buffer = Self::create_parameter_buffer(device);
        let config_buffer = Self::create_config_buffer(device, config);
        let (temp_node_buffers, final_node_buffer) = Self::create_node_buffers(device, config);
        let patch_buffer = Self::create_patch_buffer(device, config);
        let (lod_map_view, atlas_map_view) = Self::create_chunk_maps(device, config);

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
            label: "indirect_buffer".into(),
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            contents: &[0; INDIRECT_BUFFER_SIZE as usize],
        })
    }

    fn create_config_buffer(device: &RenderDevice, config: &TerrainConfig) -> Buffer {
        device.create_buffer_with_data(&BufferInitDescriptor {
            label: "config_buffer".into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: config.as_std140().as_bytes(),
        })
    }

    fn create_parameter_buffer(device: &RenderDevice) -> Buffer {
        device.create_buffer(&BufferDescriptor {
            label: "parameter_buffer".into(),
            size: PARAMETER_BUFFER_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    fn create_node_buffers(device: &RenderDevice, config: &TerrainConfig) -> ([Buffer; 2], Buffer) {
        let max_node_count = config.chunk_count.x * config.chunk_count.y;

        let buffer_descriptor = BufferDescriptor {
            label: "node_buffer".into(),
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

    fn create_patch_buffer(device: &RenderDevice, config: &TerrainConfig) -> Buffer {
        let max_patch_count =
            config.chunk_count.x * config.chunk_count.y * TerrainConfig::PATCHES_PER_NODE;

        let buffer_descriptor = BufferDescriptor {
            label: "patch_buffer".into(),
            size: PATCH_SIZE * max_patch_count as BufferAddress,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        };

        device.create_buffer(&buffer_descriptor)
    }

    fn create_chunk_maps(
        device: &RenderDevice,
        config: &TerrainConfig,
    ) -> (TextureView, TextureView) {
        let lod_map = device.create_texture(&TextureDescriptor {
            label: "lod_map".into(),
            size: Extent3d {
                width: config.chunk_count.x,
                height: config.chunk_count.y,
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
            label: "atlas_map".into(),
            size: Extent3d {
                width: config.chunk_count.x,
                height: config.chunk_count.y,
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
            label: "lod_map_view".into(),
            format: Some(TextureFormat::R8Uint),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let atlas_map_view = atlas_map.create_view(&TextureViewDescriptor {
            label: "atlas_map_view".into(),
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
            .insert(TerrainResources::new(&device, config));
    }
}
