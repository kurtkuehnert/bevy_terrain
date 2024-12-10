use bevy::render::{
    render_resource::{
        encase::internal::{ReadFrom, WriteInto},
        *,
    },
    renderer::{RenderDevice, RenderQueue},
};
use image::{ImageBuffer, Luma, LumaA, Rgb, Rgba};
use itertools::Itertools;
use smallvec::SmallVec;
use std::sync::{Arc, Mutex};
use std::{fmt::Debug, ops::Deref};

pub type Rgb8Image = ImageBuffer<Rgb<u8>, Vec<u8>>;
pub type Rgba8Image = ImageBuffer<Rgba<u8>, Vec<u8>>;
pub type R16Image = ImageBuffer<Luma<u16>, Vec<u16>>;
pub type Rg16Image = ImageBuffer<LumaA<u16>, Vec<u16>>;

pub trait CollectArray: Iterator {
    fn collect_array<const T: usize>(self) -> [Self::Item; T]
    where
        Self: Sized,
        <Self as Iterator>::Item: Debug,
    {
        self.collect_vec().try_into().unwrap()
    }
}

impl<T> CollectArray for T where T: Iterator + ?Sized {}

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

    fn read<T: ShaderType + ReadFrom>(self, value: &mut T, buffer: &impl AsRef<[u8]>) {
        match self {
            BufferType::None => {
                unimplemented!("Can not read ShaderType from BufferType::None.");
            }
            BufferType::Uniform => {
                unimplemented!("Uniform buffers can not be read.");
            }
            BufferType::Storage => {
                encase::StorageBuffer::new(buffer.as_ref())
                    .read(value)
                    .unwrap();
            }
        }
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

#[derive(Default)]
struct Pool {
    prepared: Option<Buffer>,
    in_use: SmallVec<Buffer, 3>,
    available: SmallVec<Buffer, 3>,
}

#[derive(Default)]
struct Readback {
    pool: Arc<Mutex<Pool>>,
}

pub struct GpuBuffer<T> {
    buffer: Buffer,
    pub value: Option<T>,
    buffer_type: BufferType,
    readback: Option<Readback>,
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
            readback: None,
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
            readback: None,
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
            readback: None,
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

impl<T: ShaderType + ReadFrom + Default + Send> GpuBuffer<T> {
    pub fn enable_readback(&mut self) {
        self.readback = Some(Readback::default());
    }

    /// Copy the data to the readback buffer.
    pub fn copy_to_readback(&self, device: &RenderDevice, encoder: &mut CommandEncoder) {
        let Some(readback) = &self.readback else {
            panic!()
        };

        let mut pool = readback.pool.lock().unwrap();

        let size = T::min_size().get(); // Todo: this does not work for runtime sized arrays

        let buffer = pool.available.pop().unwrap_or_else(|| {
            device.create_buffer(&BufferDescriptor {
                size,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
                label: None,
            })
        });

        encoder.copy_buffer_to_buffer(&self.buffer, 0, &buffer, 0, size);
        pool.prepared = Some(buffer);
    }

    /// Asynchronously read the contents of the readback buffer.
    ///
    /// This should only be called after all commands that update the buffer have been submitted.
    pub fn download_readback(
        &mut self,
        callback: impl FnOnce(Result<T, BufferAsyncError>) + Send + 'static,
    ) {
        let Some(readback) = &mut self.readback else {
            panic!("The buffer does not have GPU readback enabled.")
        };

        let mut pool = readback.pool.lock().unwrap();

        let Some(buffer) = pool.prepared.take() else {
            return;
        };

        pool.in_use.push(buffer.clone());

        let buffer_type = self.buffer_type;
        let pool = readback.pool.clone();

        // Todo: the downloading code should be move out of the closure, in order to avoid blocking the main thread
        buffer
            .clone()
            .slice(..)
            .map_async(MapMode::Read, move |result| {
                if let Err(e) = result {
                    callback(Err(e));
                    return;
                }

                let mut value = T::default();
                let buffer_view = buffer.slice(..).get_mapped_range();
                buffer_type.read(&mut value, &buffer_view);
                drop(buffer_view);
                buffer.unmap();

                let mut pool = pool.lock().unwrap();
                pool.in_use.retain(|other| other.id() != buffer.id());
                pool.available.push(buffer);

                callback(Ok(value));
            });
    }
}
