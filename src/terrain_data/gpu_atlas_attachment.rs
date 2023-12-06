use crate::{
    preprocess::{file_io::format_node_path, R16Image},
    preprocess_gpu::gpu_preprocessor::NodeMeta,
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
    tasks::AsyncComputeTaskPool,
};
use std::path::Path;

const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;

fn align_byte_size(value: u32) -> u32 {
    // only works for non zero values
    value - 1 - (value - 1) % COPY_BYTES_PER_ROW_ALIGNMENT + COPY_BYTES_PER_ROW_ALIGNMENT
}

async fn read_buffer(
    read_back_buffer: Buffer,
    texture_size: usize,
    pixel_size: usize,
    layer_count: usize,
) -> Vec<u8> {
    let (tx, rx) = async_channel::bounded(1);
    let buffer_slice = read_back_buffer.slice(..);
    // The polling for this map call is done every frame when the command queue is submitted.
    buffer_slice.map_async(MapMode::Read, move |result| {
        let err = result.err();
        if err.is_some() {
            panic!("{}", err.unwrap().to_string());
        }
        tx.try_send(()).unwrap();
    });
    rx.recv().await.unwrap();
    let data = buffer_slice.get_mapped_range();
    // we immediately move the data to CPU memory to avoid holding the mapped view for long
    let mut result = Vec::from(&*data);
    drop(data);
    drop(read_back_buffer);

    if result.len() != (texture_size * texture_size * pixel_size * layer_count) {
        // Our buffer has been padded because we needed to align to a multiple of 256.
        // We remove this padding here
        let initial_row_bytes = texture_size * pixel_size;
        let buffered_row_bytes = align_byte_size((texture_size * pixel_size) as u32) as usize;

        let mut take_offset = buffered_row_bytes;
        let mut place_offset = initial_row_bytes;
        for _ in 1..texture_size * layer_count {
            result.copy_within(take_offset..take_offset + buffered_row_bytes, place_offset);
            take_offset += buffered_row_bytes;
            place_offset += initial_row_bytes;
        }
        result.truncate(initial_row_bytes * texture_size);
    }

    return result;
}

#[derive(Clone, ShaderType)]
pub(crate) struct AttachmentMeta {
    pub(crate) texture_size: u32,
    pub(crate) border_size: u32,
    pub(crate) node_size: u32,
    pub(crate) pixels_per_section_entry: u32,
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
    pub(crate) read_back_buffer: Option<Buffer>,
    pub(crate) bind_group: BindGroup,
    pub(crate) format: TextureFormat,
    pub(crate) texture_size: u32,
    pub(crate) mip_level_count: u32,
    pub(crate) workgroup_count: UVec2,
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

        let read_back_size = slots
            * align_byte_size(
                attachment.texture_size * attachment.format.storage_format().pixel_size() as u32,
            )
            * attachment.texture_size;

        let read_back_buffer = Some(device.create_buffer(&BufferDescriptor {
            label: Some("read_back_buffer"),
            size: read_back_size as BufferAddress,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }));

        // Todo: adjust this code for pixel sizes larger than a u32
        let pixels_per_section_entry = std::mem::size_of::<u32>() as u32
            / attachment.format.storage_format().pixel_size() as u32;
        let attachment_meta = AttachmentMeta {
            texture_size: attachment.texture_size,
            border_size: attachment.border_size,
            node_size: attachment.texture_size - 2 * attachment.border_size,
            pixels_per_section_entry,
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
            read_back_buffer,
            bind_group,
            format: attachment.format.storage_format(),
            texture_size: attachment.texture_size,
            mip_level_count: attachment.mip_level_count,
            workgroup_count: UVec2::new(
                attachment.texture_size / 8 / pixels_per_section_entry,
                attachment.texture_size / 8,
            ),
        }
    }

    pub(crate) fn copy_nodes_to_write_section(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        nodes: &[NodeMeta],
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        for (write_section_index, node_meta) in nodes.iter().enumerate() {
            command_encoder.copy_texture_to_buffer(
                self.image_copy_texture(&atlas.texture, node_meta.atlas_index, 0),
                self.image_copy_buffer(&self.atlas_write_section, write_section_index as u32),
                self.image_copy_size(0),
            );
        }
    }

    pub(crate) fn copy_nodes_from_write_section(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        nodes: &[NodeMeta],
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        for (write_section_index, node_meta) in nodes.iter().enumerate() {
            command_encoder.copy_buffer_to_texture(
                self.image_copy_buffer(&self.atlas_write_section, write_section_index as u32),
                self.image_copy_texture(&atlas.texture, node_meta.atlas_index, 0),
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
        nodes: &[NodeMeta],
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        for (read_back_index, node_meta) in nodes.iter().enumerate() {
            command_encoder.copy_texture_to_buffer(
                self.image_copy_texture(&atlas.texture, node_meta.atlas_index as u32, 0),
                self.image_copy_buffer(
                    self.read_back_buffer.as_ref().unwrap(),
                    read_back_index as u32,
                ),
                self.image_copy_size(0),
            );
        }
    }

    pub(crate) fn save_nodes(&self, nodes: &[NodeMeta]) {
        let texture_size = self.texture_size as usize;
        let pixel_size = self.format.pixel_size();
        let layer_count = 4;

        let read_back_buffer = self.read_back_buffer.clone().unwrap();
        let nodes = nodes.to_vec();

        let finish = async move {
            let data = read_buffer(read_back_buffer, texture_size, pixel_size, layer_count).await;

            let image_size = texture_size * texture_size * pixel_size;

            for (image_data, node_meta) in data
                .chunks_exact(image_size)
                .map(|slice| {
                    slice
                        .chunks_exact(2)
                        .map(|pixel| u16::from_le_bytes(pixel.try_into().unwrap()))
                        .collect::<Vec<u16>>()
                })
                .zip(nodes.iter())
            {
                let path = format_node_path("assets/test", &node_meta.node_coordinate);
                let path = Path::new(&path);
                let path = path.with_extension("png");
                let path = path.to_str().unwrap();

                dbg!(path);

                let image =
                    R16Image::from_raw(texture_size as u32, texture_size as u32, image_data)
                        .unwrap();

                image.save(path).unwrap();

                dbg!("node data has been retreived from the GPU");
            }
        };

        AsyncComputeTaskPool::get().spawn(finish).detach();
    }
}
