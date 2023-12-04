use crate::{
    preprocess::R16Image,
    preprocess_gpu::gpu_preprocessor::NodeMeta,
    terrain_data::{AtlasAttachment, AtlasIndex},
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::{GpuImage, TextureFormatPixelInfo},
    },
    tasks::AsyncComputeTaskPool,
};

pub const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;

pub(crate) fn align_byte_size(value: u32) -> u32 {
    // only works for non zero values
    value - 1 - (value - 1) % COPY_BYTES_PER_ROW_ALIGNMENT + COPY_BYTES_PER_ROW_ALIGNMENT
}

#[derive(Clone, ShaderType)]
pub(crate) struct AttachmentMeta {
    pub(crate) texture_size: u32,
    pub(crate) border_size: u32,
    pub(crate) node_size: u32,
    pub(crate) pixels_per_section_entry: u32,
}

pub(crate) const ATTACHMENT_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        // atlas_write_section
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // atlas
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        // atlas sampler
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
        // attachment meta
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ],
};

pub(crate) struct GpuAtlasAttachment {
    pub(crate) atlas_handle: Handle<Image>,
    pub(crate) atlas_write_section: Buffer,
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

        let section_slots = 1;
        let section_size = attachment.texture_size
            * attachment.texture_size
            * section_slots
            * attachment.format.storage_format().pixel_size() as u32;
        let atlas_write_section = device.create_buffer(&BufferDescriptor {
            label: Some(&(attachment.name.to_string() + "_atlas_write_section")),
            size: section_size as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::COPY_SRC | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

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
            &device.create_bind_group_layout(&ATTACHMENT_LAYOUT),
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

    pub(crate) fn download_nodes(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        read_back_buffer: &Buffer,
        nodes: &[NodeMeta],
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        for (read_back_index, node_meta) in nodes.iter().enumerate() {
            command_encoder.copy_texture_to_buffer(
                self.image_copy_texture(&atlas.texture, node_meta.atlas_index as u32, 0),
                self.image_copy_buffer(read_back_buffer, read_back_index as u32),
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
}

pub(crate) fn save_node(read_back_buffer: Buffer) {
    let width = 512;
    let height = 512;
    let pixel_size = 2;

    let finish = async move {
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

        if result.len() != ((width * height) as usize * pixel_size) {
            // Our buffer has been padded because we needed to align to a multiple of 256.
            // We remove this padding here
            let initial_row_bytes = width as usize * pixel_size;
            let buffered_row_bytes = align_byte_size(width * pixel_size as u32) as usize;

            let mut take_offset = buffered_row_bytes;
            let mut place_offset = initial_row_bytes;
            for _ in 1..height {
                result.copy_within(take_offset..take_offset + buffered_row_bytes, place_offset);
                take_offset += buffered_row_bytes;
                place_offset += initial_row_bytes;
            }
            result.truncate(initial_row_bytes * height as usize);
        }

        let result: Vec<u16> = result
            .chunks_exact(2)
            .map(|pixel| u16::from_le_bytes(pixel.try_into().unwrap()))
            .collect();

        let image = R16Image::from_raw(width, height, result).unwrap();

        image.save("test.png").unwrap();

        dbg!("node data has been retreived from the GPU");
    };

    AsyncComputeTaskPool::get().spawn(finish).detach();
}
