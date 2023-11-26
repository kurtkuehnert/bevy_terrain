use crate::{
    terrain::{Terrain, TerrainComponents},
    terrain_data::{
        node_atlas::{LoadingNode, NodeAtlas},
        AtlasAttachment, AtlasIndex,
    },
};
use bevy::render::texture::TextureFormatPixelInfo;
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
        Extract, MainWorld,
    },
};
use std::mem;
use std::ops::Range;

pub const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;

pub(crate) fn align_byte_size(value: u32) -> u32 {
    // only works for non zero values
    value - 1 - (value - 1) % COPY_BYTES_PER_ROW_ALIGNMENT + COPY_BYTES_PER_ROW_ALIGNMENT
}

pub(crate) fn image_copy_texture(
    texture: &Texture,
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

pub(crate) fn image_copy_buffer(
    buffer: &Buffer,
    size: u32,
    format: TextureFormat,
    index: u32,
) -> ImageCopyBuffer {
    let bytes_per_row = align_byte_size(size * format.pixel_size() as u32);
    let rows_per_image = bytes_per_row * size;

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
    /// Creates the attachment from its config.
    fn create(
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

        let pixels_per_section_entry = 4 / attachment.format.storage_format().pixel_size() as u32;
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

    pub(crate) fn copy_atlas_to_rw_nodes(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        atlas_indices: Range<u32>,
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        for rw_nodes_index in atlas_indices {
            let atlas_index = rw_nodes_index;

            command_encoder.copy_texture_to_buffer(
                image_copy_texture(&atlas.texture, atlas_index, 0),
                image_copy_buffer(
                    &self.atlas_write_section,
                    self.texture_size,
                    self.format,
                    rw_nodes_index,
                ),
                Extent3d {
                    width: self.texture_size,
                    height: self.texture_size,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub(crate) fn copy_rw_nodes_to_atlas(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        atlas_indices: Range<u32>,
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        for rw_nodes_index in atlas_indices {
            let atlas_index = rw_nodes_index;

            command_encoder.copy_buffer_to_texture(
                image_copy_buffer(
                    &self.atlas_write_section,
                    self.texture_size,
                    self.format,
                    rw_nodes_index,
                ),
                image_copy_texture(&atlas.texture, atlas_index, 0),
                Extent3d {
                    width: self.texture_size,
                    height: self.texture_size,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub(crate) fn read_back_node(
        &self,
        command_encoder: &mut CommandEncoder,
        images: &RenderAssets<Image>,
        read_back_buffer: &Buffer,
        atlas_index: AtlasIndex,
    ) {
        let atlas = images.get(&self.atlas_handle).unwrap();

        let bytes_per_row = atlas.size.x as u32 * atlas.texture_format.pixel_size() as u32;
        let bytes_per_row_aligned = align_byte_size(bytes_per_row);

        dbg!(bytes_per_row);
        dbg!(bytes_per_row_aligned);

        command_encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &atlas.texture,
                mip_level: 0,
                origin: Origin3d {
                    x: 0,
                    y: 0,
                    z: atlas_index as u32,
                },
                aspect: TextureAspect::All,
            },
            ImageCopyBuffer {
                buffer: read_back_buffer,
                layout: ImageDataLayout {
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                    ..Default::default()
                },
            },
            Extent3d {
                width: atlas.size.x as u32,
                height: atlas.size.y as u32,
                depth_or_array_layers: 1,
            },
        );
    }
}

/// Stores the GPU representation of the [`NodeAtlas`] (array textures)
/// alongside the data to update it.
///
/// All attachments of newly loaded nodes are copied into their according atlas attachment.
#[derive(Component)]
pub struct GpuNodeAtlas {
    /// Stores the atlas attachments of the terrain.
    pub(crate) attachments: Vec<GpuAtlasAttachment>,
    /// Stores the nodes, that have finished loading this frame.
    pub(crate) loaded_nodes: Vec<LoadingNode>,
}

impl GpuNodeAtlas {
    /// Creates a new gpu node atlas and initializes its attachment textures.
    fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        images: &mut RenderAssets<Image>,
        node_atlas: &NodeAtlas,
    ) -> Self {
        let attachments = node_atlas
            .attachments
            .iter()
            .map(|attachment| {
                GpuAtlasAttachment::create(attachment, device, queue, images, node_atlas.size)
            })
            .collect::<Vec<_>>();

        Self {
            attachments,
            loaded_nodes: Vec::new(),
        }
    }

    /// Updates the atlas attachments, by copying over the data of the nodes that have
    /// finished loading this frame.
    fn prepare(&mut self, command_encoder: &mut CommandEncoder, images: &RenderAssets<Image>) {
        for node in self.loaded_nodes.drain(..) {
            for (node_handle, attachment) in
                self.attachments
                    .iter()
                    .enumerate()
                    .map(|(index, atlas_handle)| {
                        let node_handle = node.attachments.get(&index).unwrap();

                        (node_handle, atlas_handle)
                    })
            {
                if let (Some(node_image), Some(atlas_image)) = (
                    images.get(node_handle),
                    images.get(&attachment.atlas_handle),
                ) {
                    for mip_level in 0..attachment.mip_level_count {
                        command_encoder.copy_texture_to_texture(
                            image_copy_texture(&node_image.texture, 0, mip_level),
                            image_copy_texture(
                                &atlas_image.texture,
                                node.atlas_index as u32,
                                mip_level,
                            ),
                            Extent3d {
                                width: attachment.texture_size >> mip_level,
                                height: attachment.texture_size >> mip_level,
                                depth_or_array_layers: 1,
                            },
                        );
                    }
                } else {
                    error!("Something went wrong, attachment is not available!")
                }
            }
        }
    }
}

/// Initializes the [`GpuNodeAtlas`] of newly created terrains.
pub(crate) fn initialize_gpu_node_atlas(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut images: ResMut<RenderAssets<Image>>,
    mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
    mut terrain_query: Extract<Query<(Entity, &NodeAtlas), Added<Terrain>>>,
) {
    for (terrain, node_atlas) in terrain_query.iter_mut() {
        gpu_node_atlases.insert(
            terrain,
            GpuNodeAtlas::new(&device, &queue, &mut images, node_atlas),
        );
    }
}

/// Extracts the nodes that have finished loading from all [`NodeAtlas`]es into the
/// corresponding [`GpuNodeAtlas`]es.
pub(crate) fn extract_node_atlas(
    mut main_world: ResMut<MainWorld>,
    mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
) {
    let mut terrain_query = main_world.query::<(Entity, &mut NodeAtlas)>();

    for (terrain, mut node_atlas) in terrain_query.iter_mut(&mut main_world) {
        let gpu_node_atlas = gpu_node_atlases.get_mut(&terrain).unwrap();
        mem::swap(
            &mut node_atlas.loaded_nodes,
            &mut gpu_node_atlas.loaded_nodes,
        );
    }
}

/// Queues the attachments of the nodes that have finished loading to be copied into the
/// corresponding atlas attachments.
pub(crate) fn prepare_node_atlas(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
    mut gpu_node_atlases: ResMut<TerrainComponents<GpuNodeAtlas>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

    for terrain in terrain_query.iter() {
        let gpu_node_atlas = gpu_node_atlases.get_mut(&terrain).unwrap();
        gpu_node_atlas.prepare(&mut command_encoder, &images);
    }

    queue.submit(vec![command_encoder.finish()]);
}
