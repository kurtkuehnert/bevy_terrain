use crate::{
    preprocess_gpu::{gpu_preprocessor::ReadBackNode, preprocessor::PreprocessTask},
    terrain_data::{AtlasAttachment, AtlasIndex},
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{binding_types::*, *},
        renderer::{RenderDevice, RenderQueue},
        texture::{GpuImage, TextureFormatPixelInfo},
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use itertools::Itertools;
use std::{
    mem,
    ops::DerefMut,
    sync::{Arc, Mutex},
};

const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;

fn align_byte_size(value: u32) -> u32 {
    // only works for non zero values
    value - 1 - (value - 1) % COPY_BYTES_PER_ROW_ALIGNMENT + COPY_BYTES_PER_ROW_ALIGNMENT
}

#[derive(Clone, ShaderType)]
pub(crate) struct AttachmentMeta {
    pub(crate) texture_size: u32,
    pub(crate) border_size: u32,
    pub(crate) node_size: u32,
    pub(crate) pixels_per_entry: u32,
    pub(crate) entries_per_side: u32,
    pub(crate) entries_per_node: u32,
}

pub(crate) fn create_attachment_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                storage_buffer::<u32>(false), // atlas_write_section
                texture_2d_array(TextureSampleType::Float { filterable: true }), // atlas
                sampler(SamplerBindingType::Filtering), // atlas sampler
                uniform_buffer::<AttachmentMeta>(false), // attachment meta
            ),
        ),
    )
}

pub(crate) struct GpuAtlasAttachment {
    pub(crate) atlas_handle: Handle<Image>,
    pub(crate) atlas_write_section: Buffer,
    pub(crate) read_back_buffer: Arc<Mutex<Option<Buffer>>>,
    pub(crate) bind_group: BindGroup,
    pub(crate) format: TextureFormat,
    pub(crate) texture_size: u32,
    pub(crate) mip_level_count: u32,
    pub(crate) workgroup_count: UVec3,
}

impl GpuAtlasAttachment {
    fn image_copy_texture<'a>(
        &'a self,
        texture: &'a Texture,
        index: u32,
        mip_level: u32,
    ) -> ImageCopyTexture {
        ImageCopyTexture {
            texture,
            mip_level,
            origin: Origin3d {
                z: index,
                ..default()
            },
            aspect: TextureAspect::All,
        }
    }

    fn image_copy_buffer<'a>(&'a self, buffer: &'a Buffer, index: u32) -> ImageCopyBuffer {
        let bytes_per_row = align_byte_size(self.texture_size * self.format.pixel_size() as u32);
        let rows_per_image = self.texture_size;
        let offset = (bytes_per_row * rows_per_image * index) as BufferAddress;

        ImageCopyBuffer {
            buffer: &buffer,
            layout: ImageDataLayout {
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(rows_per_image),
                offset,
            },
        }
    }

    fn image_copy_size(&self, mip_level: u32) -> Extent3d {
        Extent3d {
            width: self.texture_size >> mip_level,
            height: self.texture_size >> mip_level,
            depth_or_array_layers: 1,
        }
    }

    pub(crate) fn create(
        attachment: &AtlasAttachment,
        device: &RenderDevice,
        queue: &RenderQueue,
        images: &mut RenderAssets<Image>,
        node_atlas_size: AtlasIndex,
    ) -> Self {
        let atlas_texture = device.create_texture(&TextureDescriptor {
            label: Some(&(attachment.name.to_string() + "_attachment")),
            size: Extent3d {
                width: attachment.texture_size,
                height: attachment.texture_size,
                depth_or_array_layers: node_atlas_size as u32,
            },
            mip_level_count: attachment.mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: attachment.format.sample_format(),
            usage: TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let atlas_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..default()
        });

        let atlas_view = atlas_texture.create_view(&default());

        // Todo: adjust this code for pixel sizes larger than a u32

        let pixels_per_entry =
            mem::size_of::<u32>() as u32 / attachment.format.storage_format().pixel_size() as u32;
        let entries_per_side = attachment.texture_size / pixels_per_entry; // has to be aligned to 256
        let entries_per_node = entries_per_side * attachment.texture_size;

        let slots = 4;

        let section_size = slots
            * attachment.texture_size
            * attachment.texture_size
            * attachment.format.storage_format().pixel_size() as u32;

        let atlas_write_section = device.create_buffer(&BufferDescriptor {
            label: Some(&(attachment.name.to_string() + "_atlas_write_section")),
            size: section_size as BufferAddress,
            usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let attachment_meta = AttachmentMeta {
            texture_size: attachment.texture_size,
            border_size: attachment.border_size,
            node_size: attachment.texture_size - 2 * attachment.border_size,
            pixels_per_entry,
            entries_per_side,
            entries_per_node,
        };

        let mut attachment_meta_buffer = UniformBuffer::from(attachment_meta);
        attachment_meta_buffer.write_buffer(device, queue);

        let bind_group = device.create_bind_group(
            "attachment_bind_group",
            &create_attachment_layout(&device),
            &BindGroupEntries::sequential((
                atlas_write_section.as_entire_binding(),
                &atlas_view,
                &atlas_sampler,
                attachment_meta_buffer.binding().unwrap(),
            )),
        );

        images.insert(
            attachment.handle.clone(),
            GpuImage {
                texture_view: atlas_view,
                texture: atlas_texture,
                texture_format: attachment.format.sample_format(),
                sampler: atlas_sampler,
                size: Vec2::splat(attachment.texture_size as f32),
                mip_level_count: attachment.mip_level_count,
            },
        );

        Self {
            atlas_handle: attachment.handle.clone(),
            atlas_write_section,
            read_back_buffer: Arc::new(Mutex::new(None)),
            bind_group,
            format: attachment.format.storage_format(),
            texture_size: attachment.texture_size,
            mip_level_count: attachment.mip_level_count,
            workgroup_count: UVec3::new(
                attachment.texture_size / 8 / pixels_per_entry,
                attachment.texture_size / 8,
                1,
            ),
        }
    }

    pub(crate) fn copy_nodes_to_write_section(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        atlas_indices: &[u32],
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        for (write_section_index, &atlas_index) in atlas_indices.iter().enumerate() {
            command_encoder.copy_texture_to_buffer(
                self.image_copy_texture(&atlas.texture, atlas_index, 0),
                self.image_copy_buffer(&self.atlas_write_section, write_section_index as u32),
                self.image_copy_size(0),
            );
        }
    }

    pub(crate) fn copy_nodes_from_write_section(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        atlas_indices: &[u32],
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        for (write_section_index, &atlas_index) in atlas_indices.iter().enumerate() {
            command_encoder.copy_buffer_to_texture(
                self.image_copy_buffer(&self.atlas_write_section, write_section_index as u32),
                self.image_copy_texture(&atlas.texture, atlas_index, 0),
                self.image_copy_size(0),
            );
        }
    }

    pub(crate) fn upload_node(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        node_handle: &Handle<Image>,
        atlas_index: AtlasIndex,
    ) {
        if let (Some(node_image), Some(atlas_image)) =
            (images.get(node_handle), images.get(&self.atlas_handle))
        {
            for mip_level in 0..self.mip_level_count {
                command_encoder.copy_texture_to_texture(
                    self.image_copy_texture(&node_image.texture, 0, mip_level),
                    self.image_copy_texture(&atlas_image.texture, atlas_index as u32, mip_level),
                    self.image_copy_size(mip_level),
                );
            }
        } else {
            error!("Something went wrong, attachment is not available!")
        }
    }

    pub(crate) fn download_nodes(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        atlas_indices: &[u32],
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        for (read_back_index, &atlas_index) in atlas_indices.iter().enumerate() {
            command_encoder.copy_texture_to_buffer(
                self.image_copy_texture(&atlas.texture, atlas_index, 0),
                self.image_copy_buffer(
                    self.read_back_buffer.lock().unwrap().as_ref().unwrap(),
                    read_back_index as u32,
                ),
                self.image_copy_size(0),
            );
        }
    }

    pub(crate) fn create_read_back_buffer(&self, device: &RenderDevice) {
        let slots = 4;

        let read_back_size = slots
            * align_byte_size(self.texture_size * self.format.pixel_size() as u32)
            * self.texture_size;

        *self.read_back_buffer.lock().unwrap().deref_mut() =
            Some(device.create_buffer(&BufferDescriptor {
                label: Some("read_back_buffer"),
                size: read_back_size as BufferAddress,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));
    }

    pub(crate) fn start_reading_back_nodes(
        &self,
        tasks: Vec<PreprocessTask>,
    ) -> Task<Vec<ReadBackNode>> {
        let texture_size = self.texture_size as usize;
        let format = self.format;
        let pixel_size = self.format.pixel_size();

        let read_back_buffer =
            mem::take(self.read_back_buffer.lock().unwrap().deref_mut()).unwrap();

        println!("Started reading back {} nodes", tasks.len());

        AsyncComputeTaskPool::get().spawn(async move {
            let (tx, rx) = async_channel::bounded(1);

            // Todo: determine how much data to read
            let buffer_slice = read_back_buffer.slice(..);

            buffer_slice.map_async(MapMode::Read, move |result| {
                if result.is_err() {
                    panic!("{}", result.err().unwrap().to_string());
                }

                tx.try_send(()).unwrap();
            });

            rx.recv().await.unwrap();

            let mapped_data = buffer_slice.get_mapped_range();

            let node_data_size = mapped_data.len() / tasks.len();

            let mut read_back_tasks = mapped_data
                .chunks_exact(node_data_size)
                .map(|node_data| node_data.to_vec())
                .zip(tasks)
                .map(|(data, task)| ReadBackNode {
                    data,
                    texture_size: texture_size as u32,
                    format,
                    meta: task.node,
                })
                .collect_vec();

            drop(mapped_data);
            read_back_buffer.unmap();
            drop(read_back_buffer);

            for node in &mut read_back_tasks {
                if node.data.len() != texture_size * texture_size * pixel_size {
                    let initial_row_bytes = texture_size * pixel_size;
                    let buffered_row_bytes =
                        align_byte_size((texture_size * pixel_size) as u32) as usize;

                    let mut take_offset = buffered_row_bytes;
                    let mut place_offset = initial_row_bytes;

                    for _ in 1..texture_size {
                        node.data.copy_within(
                            take_offset..take_offset + buffered_row_bytes,
                            place_offset,
                        );
                        take_offset += buffered_row_bytes;
                        place_offset += initial_row_bytes;
                    }

                    node.data.truncate(initial_row_bytes * texture_size);
                }
            }

            read_back_tasks
        })
    }
}
