use crate::{config::TerrainConfig, render::layouts::*};
use bevy::{
    prelude::*,
    render::{
        render_resource::{std140::Std140, *},
        renderer::RenderDevice,
    },
};

#[derive(Component)]
pub struct TerrainResources {
    pub(crate) indirect_buffer: Buffer,
    pub(crate) parameter_buffer: Buffer,
    pub(crate) config_buffer: Buffer,
    pub(crate) temp_node_buffers: [Buffer; 2],
    pub(crate) final_node_buffer: Buffer,
}

impl TerrainResources {
    pub(crate) fn new(device: &RenderDevice, config: &TerrainConfig) -> Self {
        let indirect_buffer = Self::create_indirect_buffer(device);
        let parameter_buffer = Self::create_parameter_buffer(device);
        let config_buffer = Self::create_config_buffer(device, config);
        let (temp_node_buffers, final_node_buffer) = Self::create_node_buffers(device, config);

        Self {
            indirect_buffer,
            parameter_buffer,
            config_buffer,
            temp_node_buffers,
            final_node_buffer,
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
        let buffer_descriptor = BufferDescriptor {
            label: "node_buffer".into(),
            size: 10 * NODE_SIZE * (config.chunk_count.x * config.chunk_count.y) as BufferAddress,
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
