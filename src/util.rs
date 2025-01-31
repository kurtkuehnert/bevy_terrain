use bevy::render::{
    render_resource::{encase::internal::WriteInto, *},
    renderer::{RenderDevice, RenderQueue},
};
use std::ops::Deref;

#[derive(Copy, Clone)]
enum BufferType {
    None,
    Uniform,
    Storage,
}

impl BufferType {
    fn write<T: ShaderType + WriteInto>(self, value: &T, buffer: &mut impl AsMut<[u8]>) {
        match self {
            BufferType::None => {
                unimplemented!("Can not write ShaderType to BufferType::None.");
            }
            BufferType::Uniform => {
                encase::UniformBuffer::new(buffer.as_mut())
                    .write(value)
                    .unwrap();
            }
            BufferType::Storage => {
                encase::StorageBuffer::new(buffer.as_mut())
                    .write(value)
                    .unwrap();
            }
        }
    }

    fn write_vec<T: ShaderType + WriteInto>(self, value: &T) -> Vec<u8> {
        let mut buffer = vec![0; value.size().get() as usize];
        self.write(value, &mut buffer);
        buffer
    }
}

impl From<BufferUsages> for BufferType {
    fn from(value: BufferUsages) -> Self {
        if value.contains(BufferUsages::UNIFORM) {
            BufferType::Uniform
        } else if value.contains(BufferUsages::STORAGE) {
            BufferType::Storage
        } else {
            BufferType::None
        }
    }
}

pub struct GpuBuffer<T> {
    buffer: Buffer,
    pub value: Option<T>,
    buffer_type: BufferType,
}

impl<T> GpuBuffer<T> {
    pub fn empty_sized_labeled<'a>(
        label: impl Into<Option<&'a str>>,
        device: &RenderDevice,
        size: BufferAddress,
        usage: BufferUsages,
    ) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: label.into(),
            size,
            usage,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            value: None,
            buffer_type: usage.into(),
        }
    }

    pub fn empty_sized(device: &RenderDevice, size: BufferAddress, usage: BufferUsages) -> Self {
        Self::empty_sized_labeled(None, device, size, usage)
    }

    pub fn update_bytes(&self, queue: &RenderQueue, bytes: &[u8]) {
        queue.write_buffer(&self.buffer, 0, bytes);
    }
}

impl<T: ShaderType + Default> GpuBuffer<T> {
    pub fn empty_labeled<'a>(
        label: impl Into<Option<&'a str>>,
        device: &RenderDevice,
        usage: BufferUsages,
    ) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: label.into(),
            size: T::min_size().get(),
            usage,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            value: None,
            buffer_type: usage.into(),
        }
    }

    pub fn empty(device: &RenderDevice, usage: BufferUsages) -> Self {
        Self::empty_labeled(None, device, usage)
    }
}

impl<T: ShaderType + WriteInto> GpuBuffer<T> {
    pub fn create_labeled<'a>(
        label: impl Into<Option<&'a str>>,
        device: &RenderDevice,
        value: &T,
        usage: BufferUsages,
    ) -> Self {
        let buffer_type: BufferType = usage.into();
        let contents = buffer_type.write_vec(value);

        let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: label.into(),
            usage,
            contents: &contents,
        });

        Self {
            buffer,
            value: None,
            buffer_type,
        }
    }

    pub fn create(device: &RenderDevice, value: &T, usage: BufferUsages) -> Self {
        Self::create_labeled(None, device, value, usage)
    }

    pub fn value(&self) -> &T {
        self.value.as_ref().unwrap()
    }

    pub fn set_value(&mut self, value: T) {
        self.value = Some(value);
    }

    pub fn update(&mut self, queue: &RenderQueue) {
        if let Some(value) = &self.value {
            let mut buffer = queue
                .write_buffer_with(&self.buffer, 0, value.size())
                .unwrap();
            self.buffer_type.write(value, &mut buffer);
        }
    }
}

impl<T> Deref for GpuBuffer<T> {
    type Target = Buffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<'a, T> IntoBinding<'a> for &'a GpuBuffer<T> {
    #[inline]
    fn into_binding(self) -> BindingResource<'a> {
        self.buffer.as_entire_binding()
    }
}
