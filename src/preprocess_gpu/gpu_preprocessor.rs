use crate::preprocess::TileConfig;
use crate::preprocess_gpu::preprocessor::Preprocessor;
use crate::terrain::{Terrain, TerrainComponents};
use crate::terrain_data::gpu_atlas_attachment::save_node;
use crate::terrain_data::NodeCoordinate;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::Extract;

// Todo: this does not belong here
#[derive(Clone, ShaderType)]
pub(crate) struct NodeMeta {
    pub(crate) atlas_index: u32,
    pub(crate) node_coordinate: NodeCoordinate,
}

pub(crate) const PREPROCESS_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        // tile
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        // tile_sampler
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
        // node_meta_list
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ],
};

pub(crate) struct GpuPreprocessor {
    _tile_config: Option<TileConfig>,
    tile_handle: Option<Handle<Image>>,
    pub(crate) affected_nodes: Vec<NodeMeta>,
    pub(crate) is_ready: bool,
    pub(crate) preprocess_bind_group: Option<BindGroup>,
    pub(crate) read_back_buffer: Option<Buffer>,
}

impl GpuPreprocessor {
    pub(crate) fn new(preprocessor: &Preprocessor) -> Self {
        Self {
            _tile_config: preprocessor.tile_config.clone(),
            tile_handle: preprocessor.tile_handle.clone(),
            affected_nodes: preprocessor.affected_nodes.clone(),
            is_ready: preprocessor.is_ready,
            preprocess_bind_group: None,
            read_back_buffer: None,
        }
    }

    pub(crate) fn update(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        images: &RenderAssets<Image>,
    ) {
        if self.is_ready {
            let mut nodes_meta_buffer = StorageBuffer::from(self.affected_nodes.clone());
            nodes_meta_buffer.write_buffer(device, queue);

            let tile = images.get(self.tile_handle.as_ref().unwrap()).unwrap();

            let preprocess_bind_group = device.create_bind_group(
                "preprocess_bind_group",
                &device.create_bind_group_layout(&PREPROCESS_LAYOUT),
                &BindGroupEntries::sequential((
                    &tile.texture_view,
                    &tile.sampler,
                    nodes_meta_buffer.binding().unwrap(),
                )),
            );

            self.read_back_buffer = Some(device.create_buffer(&BufferDescriptor {
                label: Some("read_back_buffer"),
                size: 512 * 512 * 2 * 4,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }));

            self.preprocess_bind_group = Some(preprocess_bind_group);
        }
    }

    pub(crate) fn extract(
        mut preprocess_data: ResMut<TerrainComponents<GpuPreprocessor>>,
        terrain_query: Extract<Query<(Entity, &Preprocessor), With<Terrain>>>,
    ) {
        for (entity, preprocessor) in terrain_query.iter() {
            if let Some(data) = preprocess_data.get(&entity) {
                if data.is_ready {
                    save_node(data.read_back_buffer.clone().unwrap());
                }
            }
            preprocess_data.insert(entity, GpuPreprocessor::new(&preprocessor));
        }
    }

    pub(crate) fn prepare(
        device: Res<RenderDevice>,
        queue: Res<RenderQueue>,
        images: Res<RenderAssets<Image>>,
        mut preprocess_data: ResMut<TerrainComponents<GpuPreprocessor>>,
    ) {
        for data in preprocess_data.0.values_mut() {
            data.update(&device, &queue, &images);
        }
    }
}
