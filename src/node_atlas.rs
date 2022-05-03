use crate::render::layouts::NODE_UPDATE_SIZE;
use crate::render::resources::NodeAttachment;
use crate::render::{InitTerrain, PersistentComponent};
use crate::{config::TerrainConfig, quadtree::NodeData};
use bevy::render::texture::GpuImage;
use bevy::render::RenderWorld;
use bevy::utils::HashMap;
use bevy::{
    core::{cast_slice, Pod, Zeroable},
    ecs::{
        query::QueryItem,
        system::lifetimeless::{Read, Write},
    },
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_component::ExtractComponent,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
    },
};
use std::num::NonZeroU32;
use std::ops::Deref;
use std::{collections::VecDeque, mem};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
pub(crate) struct NodeUpdate {
    pub(crate) node_id: u32,
    pub(crate) atlas_index: u32, // u16 not supported by std 140
}

/// Maps the assets to the corresponding active nodes and tracks the node updates.
#[derive(Component)]
pub struct NodeAtlas {
    pub(crate) available_indices: VecDeque<u16>,
    quadtree_update: Vec<Vec<NodeUpdate>>,
    activated_nodes: Vec<(u16, NodeData)>,
}

impl NodeAtlas {
    // pub(crate) const NONEXISTING_ID: u16 = u16::MAX;
    pub(crate) const INACTIVE_ID: u16 = u16::MAX - 1;

    pub fn new(config: &TerrainConfig) -> Self {
        Self {
            available_indices: (0..config.node_atlas_size).collect(),
            quadtree_update: vec![Vec::new(); config.lod_count as usize],
            activated_nodes: default(),
        }
    }

    pub(crate) fn activate_node(&mut self, node: &mut NodeData) {
        let atlas_index = self
            .available_indices
            .pop_front()
            .expect("Out of atlas ids.");

        node.atlas_index = atlas_index;

        self.activated_nodes.push((atlas_index, node.clone()));

        let lod = TerrainConfig::node_position(node.id).0 as usize;
        self.quadtree_update[lod].push(NodeUpdate {
            node_id: node.id,
            atlas_index: node.atlas_index as u32,
        });
    }

    pub(crate) fn deactivate_node(&mut self, node: &mut NodeData) {
        self.available_indices.push_front(node.atlas_index);

        node.atlas_index = Self::INACTIVE_ID;

        let lod = TerrainConfig::node_position(node.id).0 as usize;
        self.quadtree_update[lod].push(NodeUpdate {
            node_id: node.id,
            atlas_index: node.atlas_index as u32,
        });
    }
}

pub struct GpuNodeAtlas {
    pub(crate) quadtree_view: TextureView,
    pub(crate) quadtree_update_buffers: Vec<Buffer>,
    pub(crate) quadtree_views: Vec<TextureView>,
    pub(crate) node_update_counts: Vec<u32>,
    pub(crate) quadtree_update: Vec<Vec<NodeUpdate>>,
    pub(crate) atlas_attachments: HashMap<String, NodeAttachment>,
    pub(crate) activated_nodes: Vec<(u16, NodeData)>, // make generic on NodeData
    pub(crate) height_atlas: GpuImage,
}

impl GpuNodeAtlas {
    fn new(config: &TerrainConfig, device: &RenderDevice, queue: &RenderQueue) -> Self {
        let (quadtree_view, quadtree_update_buffers, quadtree_views) =
            Self::create_quadtree(config, device, queue);

        let height_atlas = Self::create_node_atlas(config, device);

        Self {
            quadtree_view,
            quadtree_update_buffers,
            quadtree_views,
            node_update_counts: vec![],
            quadtree_update: vec![],
            atlas_attachments: Default::default(),
            activated_nodes: vec![],
            height_atlas,
        }
    }
    fn create_quadtree(
        config: &TerrainConfig,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) -> (TextureView, Vec<Buffer>, Vec<TextureView>) {
        let texture_descriptor = TextureDescriptor {
            label: None,
            size: Extent3d {
                width: config.chunk_count.x,
                height: config.chunk_count.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: config.lod_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Uint,
            usage: TextureUsages::COPY_DST
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING,
        };

        let quadtree_texture = device.create_texture(&texture_descriptor);

        // Todo: use https://docs.rs/wgpu/latest/wgpu/util/trait.DeviceExt.html#tymethod.create_buffer_init once its added to bevy

        for lod in 0..config.lod_count {
            let node_count = config.node_count(lod);

            let texture = ImageCopyTextureBase {
                texture: quadtree_texture.deref(),
                mip_level: lod,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            };

            let data_layout = ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::try_from(node_count.x * 2).unwrap()),
                rows_per_image: Some(NonZeroU32::try_from(node_count.y).unwrap()),
            };

            let size = Extent3d {
                width: node_count.x,
                height: node_count.y,
                depth_or_array_layers: 1,
            };

            let data: Vec<u16> =
                vec![NodeAtlas::INACTIVE_ID; (node_count.x * node_count.y) as usize];

            queue.write_texture(texture, cast_slice(&data), data_layout, size);
        }

        let quadtree_view = quadtree_texture.create_view(&TextureViewDescriptor {
            label: None,
            format: Some(TextureFormat::R16Uint),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let (quadtree_buffers, quadtree_views) = (0..config.lod_count)
            .map(|lod| {
                let node_count = config.node_count(lod);
                let max_node_count = (node_count.x * node_count.y) as BufferAddress;

                let buffer = device.create_buffer(&BufferDescriptor {
                    label: None,
                    size: NODE_UPDATE_SIZE * max_node_count,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let view = quadtree_texture.create_view(&TextureViewDescriptor {
                    label: None,
                    format: Some(TextureFormat::R16Uint),
                    dimension: Some(TextureViewDimension::D2),
                    aspect: TextureAspect::All,
                    base_mip_level: lod,
                    mip_level_count: NonZeroU32::new(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                });

                (buffer, view)
            })
            .unzip();

        (quadtree_view, quadtree_buffers, quadtree_views)
    }

    fn create_node_atlas(config: &TerrainConfig, device: &RenderDevice) -> GpuImage {
        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: config.texture_size,
                height: config.texture_size,
                depth_or_array_layers: config.node_atlas_size as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
            compare: None,
            anisotropy_clamp: None,
            border_color: None,
        });

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: None,
            format: None,
            dimension: Some(TextureViewDimension::D2Array),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        // Todo: consider using custom struct with only texture and view instead
        let height_atlas = GpuImage {
            texture,
            texture_view,
            texture_format: TextureFormat::R16Unorm,
            sampler,
            size: Size::new(config.texture_size as f32, config.texture_size as f32),
        };

        height_atlas
    }
}

pub(crate) fn extract_node_atlas(
    mut render_world: ResMut<RenderWorld>,
    mut terrain_query: Query<(Entity, &mut NodeAtlas, &TerrainConfig), ()>,
) {
    let mut gpu_node_atlases = render_world.resource_mut::<PersistentComponent<GpuNodeAtlas>>();

    for (entity, mut node_atlas, config) in terrain_query.iter_mut() {
        let gpu_node_atlas = match gpu_node_atlases.get_mut(&entity) {
            Some(gpu_node_atlas) => gpu_node_atlas,
            None => continue,
        };

        gpu_node_atlas.node_update_counts.clear();
        gpu_node_atlas.activated_nodes = mem::take(&mut node_atlas.activated_nodes);
        gpu_node_atlas.quadtree_update = mem::replace(
            &mut node_atlas.quadtree_update,
            vec![Vec::new(); config.lod_count as usize],
        );
    }
}

/// Runs in prepare.
pub(crate) fn init_node_atlas(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut gpu_node_atlases: ResMut<PersistentComponent<GpuNodeAtlas>>,
    terrain_query: Query<(Entity, &TerrainConfig), With<InitTerrain>>,
) {
    for (entity, config) in terrain_query.iter() {
        info!("initializing gpu node atlas");

        gpu_node_atlases.insert(entity, GpuNodeAtlas::new(config, &device, &queue));
    }
}

pub(crate) fn queue_node_atlas_updates(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
    mut gpu_node_atlases: ResMut<PersistentComponent<GpuNodeAtlas>>,
    terrain_query: Query<(Entity, &TerrainConfig), ()>,
) {
    let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

    for (entity, config) in terrain_query.iter() {
        let gpu_node_atlas = gpu_node_atlases.get_mut(&entity).unwrap();

        for (index, node_data) in &gpu_node_atlas.activated_nodes {
            let image = images.get(&node_data.height_map).unwrap();

            command_encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &image.texture,
                    mip_level: 0,
                    origin: Origin3d { x: 0, y: 0, z: 0 },
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &gpu_node_atlas.height_atlas.texture,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: *index as u32,
                    },
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: config.texture_size,
                    height: config.texture_size,
                    depth_or_array_layers: 1,
                },
            );
        }

        let counts = gpu_node_atlas
            .quadtree_update_buffers
            .iter()
            .zip(&gpu_node_atlas.quadtree_update)
            .map(|(buffer, quadtree_update)| {
                queue.write_buffer(buffer, 0, cast_slice(quadtree_update));
                quadtree_update.len() as u32
            })
            .collect();

        gpu_node_atlas.node_update_counts = counts;
    }

    queue.submit(vec![command_encoder.finish()]);
}
