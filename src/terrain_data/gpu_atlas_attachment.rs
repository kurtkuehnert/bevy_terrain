use crate::{
    preprocess_gpu::gpu_preprocessor::NodeMeta,
    terrain_data::{node_atlas::ReadBackNode, AtlasAttachment, AttachmentFormat},
    util::StaticBuffer,
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{binding_types::*, *},
        renderer::RenderDevice,
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use itertools::Itertools;
use std::mem;

const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;

fn align_byte_size(value: u32) -> u32 {
    // only works for non zero values
    value - 1 - (value - 1) % COPY_BYTES_PER_ROW_ALIGNMENT + COPY_BYTES_PER_ROW_ALIGNMENT
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

#[derive(Default, ShaderType)]
pub(crate) struct AttachmentMeta {
    pub(crate) texture_size: u32,
    pub(crate) border_size: u32,
    pub(crate) node_size: u32,
    pub(crate) pixels_per_entry: u32,
    pub(crate) entries_per_side: u32,
    pub(crate) entries_per_node: u32,
}

#[derive(Clone)]
pub(crate) struct AtlasBufferInfo {
    pixels_per_entry: u32,
    pixels_per_side: u32,
    entries_per_side: u32,
    entries_per_node: u32,

    actual_side_size: u32,
    aligned_side_size: u32,
    actual_node_size: u32,
    aligned_node_size: u32,
}

impl AtlasBufferInfo {
    fn new(texture_size: u32, format: AttachmentFormat) -> Self {
        // Todo: adjust this code for pixel sizes larger than 4 byte
        // This approach is currently limited to 1, 2, and 4 byte sized pixels
        // Extending it to 8 and 16 sized pixels should be quite easy.
        // However 3, 6, 12 sized pixels do and will not work!
        // For them to work properly we will need to write into a texture instead of buffer.

        let pixel_size = format.pixel_size();
        let entry_size = mem::size_of::<u32>() as u32;
        let pixels_per_entry = entry_size / pixel_size;
        let pixels_per_side = texture_size;
        let entries_per_side = align_byte_size(pixels_per_side * pixel_size) / pixels_per_entry;
        let entries_per_node = entries_per_side * pixels_per_side;

        let actual_side_size = pixels_per_side * pixel_size;
        let aligned_side_size = entries_per_side * entry_size;
        let actual_node_size = pixels_per_side * pixels_per_side * pixel_size;
        let aligned_node_size = entries_per_node * entry_size;

        Self {
            pixels_per_side,
            pixels_per_entry,
            entries_per_side,
            entries_per_node,
            actual_side_size,
            aligned_side_size,
            actual_node_size,
            aligned_node_size,
        }
    }

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
        ImageCopyBuffer {
            buffer,
            layout: ImageDataLayout {
                bytes_per_row: Some(self.aligned_side_size),
                rows_per_image: Some(self.pixels_per_side),
                offset: self.buffer_size(index) as BufferAddress,
            },
        }
    }

    fn image_copy_size(&self, mip_level: u32) -> Extent3d {
        Extent3d {
            width: self.pixels_per_side >> mip_level,
            height: self.pixels_per_side >> mip_level,
            depth_or_array_layers: 1,
        }
    }

    fn buffer_size(&self, slots: u32) -> u32 {
        slots * self.aligned_node_size
    }
}

pub(crate) struct GpuAtlasAttachment {
    pub(crate) max_slots: usize,
    pub(crate) slots: Vec<NodeMeta>,
    pub(crate) atlas_texture: Texture,
    pub(crate) atlas_view: TextureView,
    pub(crate) atlas_write_section: StaticBuffer<()>,
    pub(crate) read_back_buffer: Option<StaticBuffer<()>>,
    pub(crate) bind_group: BindGroup,
    pub(crate) buffer_info: AtlasBufferInfo,
    pub(crate) mip_level_count: u32,
    pub(crate) workgroup_count: UVec3,
}

impl GpuAtlasAttachment {
    pub(crate) fn create(
        attachment: &AtlasAttachment,
        device: &RenderDevice,
        node_atlas_size: u32,
    ) -> Self {
        let max_slots = 16;

        let atlas_texture = device.create_texture(&TextureDescriptor {
            label: Some(&(attachment.name.to_string() + "_attachment")),
            size: Extent3d {
                width: attachment.texture_size,
                height: attachment.texture_size,
                depth_or_array_layers: node_atlas_size,
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

        let atlas_view = atlas_texture.create_view(&default());

        let atlas_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..default()
        });

        let buffer_info = AtlasBufferInfo::new(attachment.texture_size, attachment.format);

        let atlas_write_section = StaticBuffer::empty_sized(
            device,
            buffer_info.buffer_size(max_slots as u32) as BufferAddress,
            BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
        );

        let attachment_meta = AttachmentMeta {
            texture_size: attachment.texture_size,
            border_size: attachment.border_size,
            node_size: attachment.texture_size - 2 * attachment.border_size,
            pixels_per_entry: buffer_info.pixels_per_entry,
            entries_per_side: buffer_info.entries_per_side,
            entries_per_node: buffer_info.entries_per_node,
        };

        let attachment_meta_buffer =
            StaticBuffer::create(device, &attachment_meta, BufferUsages::UNIFORM);

        let bind_group = device.create_bind_group(
            "attachment_bind_group",
            &create_attachment_layout(device),
            &BindGroupEntries::sequential((
                &atlas_write_section,
                &atlas_view,
                &atlas_sampler,
                &attachment_meta_buffer,
            )),
        );

        let workgroup_count = UVec3::new(
            buffer_info.entries_per_side / 8,
            buffer_info.pixels_per_side / 8,
            1,
        );

        Self {
            slots: vec![],
            max_slots,
            atlas_texture,
            atlas_view,
            atlas_write_section,
            read_back_buffer: None,
            bind_group,
            mip_level_count: attachment.mip_level_count,
            workgroup_count,
            buffer_info,
        }
    }

    pub(crate) fn reserve_write_slot(&mut self, node_meta: NodeMeta) -> Option<u32> {
        if self.slots.len() < self.max_slots - 1 {
            self.slots.push(node_meta);
            Some(self.slots.len() as u32 - 1)
        } else {
            None
        }
    }

    pub(crate) fn copy_nodes_to_write_section(&self, command_encoder: &mut CommandEncoder) {
        for (section_index, node_meta) in self.slots.iter().enumerate() {
            command_encoder.copy_texture_to_buffer(
                self.buffer_info
                    .image_copy_texture(&self.atlas_texture, node_meta.atlas_index, 0),
                self.buffer_info
                    .image_copy_buffer(&self.atlas_write_section, section_index as u32),
                self.buffer_info.image_copy_size(0),
            );
        }
    }

    pub(crate) fn copy_nodes_from_write_section(&self, command_encoder: &mut CommandEncoder) {
        for (section_index, node_meta) in self.slots.iter().enumerate() {
            command_encoder.copy_buffer_to_texture(
                self.buffer_info
                    .image_copy_buffer(&self.atlas_write_section, section_index as u32),
                self.buffer_info
                    .image_copy_texture(&self.atlas_texture, node_meta.atlas_index, 0),
                self.buffer_info.image_copy_size(0),
            );
        }
    }

    pub(crate) fn upload_node(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        node_handle: &Handle<Image>,
        atlas_index: u32,
    ) {
        if let Some(node_image) = images.get(node_handle) {
            for mip_level in 0..self.mip_level_count {
                command_encoder.copy_texture_to_texture(
                    self.buffer_info
                        .image_copy_texture(&node_image.texture, 0, mip_level),
                    self.buffer_info.image_copy_texture(
                        &self.atlas_texture,
                        atlas_index,
                        mip_level,
                    ),
                    self.buffer_info.image_copy_size(mip_level),
                );
            }
        } else {
            error!("Something went wrong, attachment is not available!")
        }
    }

    // Todo: combine with read back nodes?
    pub(crate) fn download_nodes(&self, command_encoder: &mut CommandEncoder) {
        for (read_back_index, node_meta) in self.slots.iter().enumerate() {
            command_encoder.copy_texture_to_buffer(
                self.buffer_info
                    .image_copy_texture(&self.atlas_texture, node_meta.atlas_index, 0),
                self.buffer_info.image_copy_buffer(
                    self.read_back_buffer.as_ref().unwrap(),
                    read_back_index as u32,
                ),
                self.buffer_info.image_copy_size(0),
            );
        }
    }

    pub(crate) fn create_read_back_buffer(&mut self, device: &RenderDevice) {
        self.read_back_buffer = Some(StaticBuffer::empty_sized(
            device,
            self.buffer_info.buffer_size(self.slots.len() as u32) as BufferAddress,
            BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        ));
    }

    pub(crate) fn start_reading_back_nodes(&mut self) -> Task<Vec<ReadBackNode>> {
        let buffer_info = self.buffer_info.clone();
        let read_back_buffer = mem::take(&mut self.read_back_buffer).unwrap();
        let slots = mem::take(&mut self.slots);

        println!("Started reading back {} nodes", slots.len());

        AsyncComputeTaskPool::get().spawn(async move {
            let (tx, rx) = async_channel::bounded(1);

            let buffer_slice = read_back_buffer.slice(..);

            buffer_slice.map_async(MapMode::Read, move |result| {
                if result.is_err() {
                    panic!("{}", result.err().unwrap().to_string());
                }

                tx.try_send(()).unwrap();
            });

            rx.recv().await.unwrap();

            let mapped_data = buffer_slice.get_mapped_range();

            let mut read_back_nodes = mapped_data
                .chunks_exact(buffer_info.aligned_node_size as usize)
                .map(|node_data| node_data.to_vec())
                .zip(slots)
                .map(|(data, node_meta)| ReadBackNode {
                    meta: node_meta,
                    data,
                    texture_size: buffer_info.pixels_per_side,
                })
                .collect_vec();

            drop(mapped_data);
            read_back_buffer.unmap();
            drop(read_back_buffer);

            for node in &mut read_back_nodes {
                if node.data.len() != buffer_info.actual_node_size as usize {
                    let actual_side_size = buffer_info.actual_side_size as usize;
                    let aligned_side_size = buffer_info.aligned_side_size as usize;

                    let mut take_offset = aligned_side_size;
                    let mut place_offset = actual_side_size;

                    for _ in 1..buffer_info.pixels_per_side {
                        node.data.copy_within(
                            take_offset..take_offset + aligned_side_size,
                            place_offset,
                        );
                        take_offset += aligned_side_size;
                        place_offset += actual_side_size;
                    }

                    node.data.truncate(buffer_info.actual_node_size as usize);
                }
            }

            read_back_nodes
        })
    }
}
