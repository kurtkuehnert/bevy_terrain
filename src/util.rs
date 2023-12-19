use bevy::render::render_resource::encase::internal::WriteInto;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};

pub(crate) struct StaticUniformBuffer<T: ShaderType>(UniformBuffer<T>);

impl<T: ShaderType + WriteInto> StaticUniformBuffer<T> {
    pub(crate) fn create(value: T, device: &RenderDevice, queue: &RenderQueue) -> Self {
        let mut buffer = UniformBuffer::from(value);
        buffer.write_buffer(device, queue);

        Self(buffer)
    }

    pub(crate) fn binding(&self) -> BindingResource {
        self.0.binding().unwrap()
    }
}
