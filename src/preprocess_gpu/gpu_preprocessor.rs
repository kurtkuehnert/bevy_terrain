use crate::{
    preprocess::TileConfig,
    preprocess_gpu::preprocessor::Preprocessor,
    terrain::{Terrain, TerrainComponents},
    terrain_data::{gpu_node_atlas::GpuNodeAtlas, NodeCoordinate},
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{binding_types::*, *},
        renderer::{RenderDevice, RenderQueue},
        Extract,
    },
};

// Todo: this does not belong here
#[derive(Clone, Debug, ShaderType)]
pub(crate) struct NodeMeta {
    pub(crate) atlas_index: u32,
    pub(crate) _padding: u32,
    pub(crate) node_coordinate: NodeCoordinate,
}

pub(crate) fn create_preprocess_layout(device: &RenderDevice) -> BindGroupLayout {
    device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_2d(TextureSampleType::Float { filterable: true }), // tile
                sampler(SamplerBindingType::Filtering),                    // tile_sampler
                storage_buffer_read_only::<NodeMeta>(false),               // node_meta_list
            ),
        ),
    )
}

pub(crate) struct GpuPreprocessor {
    _tile_config: Option<TileConfig>,
    tile_handle: Option<Handle<Image>>,
    pub(crate) affected_nodes: Vec<NodeMeta>,
    pub(crate) is_ready: bool,
    pub(crate) preprocess_bind_group: Option<BindGroup>,
}

impl GpuPreprocessor {
    pub(crate) fn new(preprocessor: &Preprocessor) -> Self {
        Self {
            _tile_config: preprocessor.tile_config.clone(),
            tile_handle: preprocessor.tile_handle.clone(),
            affected_nodes: preprocessor.affected_nodes.clone(),
            is_ready: preprocessor.is_ready,
            preprocess_bind_group: None,
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
                &create_preprocess_layout(device),
                &BindGroupEntries::sequential((
                    &tile.texture_view,
                    &tile.sampler,
                    nodes_meta_buffer.binding().unwrap(),
                )),
            );

            self.preprocess_bind_group = Some(preprocess_bind_group);
        }
    }

    pub(crate) fn extract(
        mut preprocess_data: ResMut<TerrainComponents<GpuPreprocessor>>,
        gpu_node_atlases: Res<TerrainComponents<GpuNodeAtlas>>,
        terrain_query: Extract<Query<(Entity, &Preprocessor), With<Terrain>>>,
    ) {
        for (terrain, preprocessor) in terrain_query.iter() {
            if let Some(data) = preprocess_data.get(&terrain) {
                if data.is_ready {
                    let gpu_node_atlas = gpu_node_atlases.get(&terrain).unwrap();
                    let attachment = &gpu_node_atlas.attachments[0];

                    let nodes = &data.affected_nodes[0..4];

                    attachment.save_nodes(nodes);
                }
            }
            preprocess_data.insert(terrain, GpuPreprocessor::new(&preprocessor));
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
