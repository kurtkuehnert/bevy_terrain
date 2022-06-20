use crate::{config::TerrainConfig, render::layouts::*};
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
};

#[derive(Component)]
pub struct TerrainResources {
    pub(crate) indirect_buffer: Buffer,
    pub(crate) parameter_buffer: Buffer,
    pub(crate) config_buffer: Buffer,
    pub(crate) temp_patch_buffers: [Buffer; 2],
    pub(crate) final_patch_buffer: Buffer,
}

impl TerrainResources {
    pub(crate) fn new(device: &RenderDevice, config: &TerrainConfig) -> Self {
        let indirect_buffer = Self::create_indirect_buffer(device);
        let parameter_buffer = Self::create_parameter_buffer(device);
        let config_buffer = Self::create_config_buffer(device, config);
        let (temp_patch_buffers, final_patch_buffer) = Self::create_patch_buffers(device, config);

        Self {
            indirect_buffer,
            parameter_buffer,
            config_buffer,
            temp_patch_buffers,
            final_patch_buffer,
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
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(&config.shader_data()).unwrap();

        device.create_buffer_with_data(&BufferInitDescriptor {
            label: "config_buffer".into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: &buffer.into_inner(),
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

    fn create_patch_buffers(
        device: &RenderDevice,
        config: &TerrainConfig,
    ) -> ([Buffer; 2], Buffer) {
        let buffer_descriptor = BufferDescriptor {
            label: "patch_buffer".into(),
            size: PATCH_SIZE * config.patch_count as BufferAddress,
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
}

/// Runs in prepare.
pub(crate) fn initialize_terrain_resources(
    mut commands: Commands,
    device: Res<RenderDevice>,
    terrain_query: Query<(Entity, &TerrainConfig)>,
) {
    for (entity, config) in terrain_query.iter() {
        commands
            .get_or_spawn(entity)
            .insert(TerrainResources::new(&device, config));
    }
}
