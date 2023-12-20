use bevy::render::{
    render_resource::encase::internal::WriteInto,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
};
use std::{marker::PhantomData, ops::Deref};

enum Scratch {
    None,
    Uniform(encase::UniformBuffer<Vec<u8>>),
    Storage(encase::StorageBuffer<Vec<u8>>),
}

impl Scratch {
    fn new(usage: BufferUsages) -> Self {
        if usage.contains(BufferUsages::UNIFORM) {
            Self::Uniform(encase::UniformBuffer::new(Vec::new()))
        } else if usage.contains(BufferUsages::STORAGE) {
            Self::Storage(encase::StorageBuffer::new(Vec::new()))
        } else {
            Self::None
        }
    }

    fn write<T: ShaderType + WriteInto>(&mut self, value: &T) {
        match self {
            Scratch::None => panic!("Can't write to an buffer without a scratch buffer."),
            Scratch::Uniform(scratch) => scratch.write(value).unwrap(),
            Scratch::Storage(scratch) => scratch.write(value).unwrap(),
        }
    }

    fn contents(&self) -> &[u8] {
        match self {
            Scratch::None => panic!("Can't get the contents of a buffer without a scratch buffer."),
            Scratch::Uniform(scratch) => scratch.as_ref(),
            Scratch::Storage(scratch) => scratch.as_ref(),
        }
    }
}

pub struct StaticBuffer<T> {
    buffer: Buffer,
    scratch: Scratch,
    _marker: PhantomData<T>,
}

impl StaticBuffer<()> {
    pub fn empty_sized(device: &RenderDevice, size: BufferAddress, usage: BufferUsages) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            scratch: Scratch::None,
            _marker: PhantomData::default(),
        }
    }
}

impl<T: ShaderType + Default> StaticBuffer<T> {
    pub fn empty(device: &RenderDevice, usage: BufferUsages) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: T::min_size().get(),
            usage,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            scratch: Scratch::new(usage),
            _marker: PhantomData::default(),
        }
    }
}

impl<T: ShaderType + WriteInto> StaticBuffer<T> {
    pub fn create(device: &RenderDevice, value: &T, usage: BufferUsages) -> Self {
        let mut scratch = Scratch::new(usage);
        scratch.write(&value);

        let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: None,
            usage,
            contents: scratch.contents(),
        });

        Self {
            buffer,
            scratch,
            _marker: PhantomData::default(),
        }
    }

    pub fn update(&mut self, queue: &RenderQueue, value: &T) {
        self.scratch.write(&value);

        queue.write_buffer(&self.buffer, 0, self.scratch.contents());
    }
}

impl<T> Deref for StaticBuffer<T> {
    type Target = Buffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<'a, T> IntoBinding<'a> for &'a StaticBuffer<T> {
    #[inline]
    fn into_binding(self) -> BindingResource<'a> {
        self.buffer.as_entire_binding()
    }
}
