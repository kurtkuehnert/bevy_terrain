use crate::{config::TerrainConfig, quadtree::NodeData, TerrainData};
use bevy::{
    core::{Pod, Zeroable},
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
use bytemuck::cast_slice;
use std::collections::VecDeque;
use std::mem;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
pub(crate) struct NodeUpdate {
    pub(crate) node_id: u32,
    pub(crate) atlas_index: u32, // u16 not supported by std 140
}

#[derive(Component)]
pub struct GpuNodeAtlas {
    pub(crate) node_update_counts: Vec<u32>,
    quadtree_update: Vec<Vec<NodeUpdate>>,
    activated_height_maps: Vec<(u16, Handle<Image>)>,
}

/// Maps the assets to the corresponding active nodes and tracks the node updates.
#[derive(Component)]
pub struct NodeAtlas {
    pub(crate) available_ids: VecDeque<u16>,
    quadtree_update: Vec<Vec<NodeUpdate>>,
    activated_height_maps: Vec<(u16, Handle<Image>)>,
}

impl NodeAtlas {
    // pub(crate) const NONEXISTING_ID: u16 = u16::MAX;
    pub(crate) const INACTIVE_ID: u16 = u16::MAX - 1;

    pub fn new(config: &TerrainConfig) -> Self {
        Self {
            available_ids: (0..config.node_atlas_size).collect(),
            quadtree_update: vec![Vec::new(); config.lod_count as usize],
            activated_height_maps: Default::default(),
        }
    }

    pub(crate) fn activate_node(&mut self, node: &mut NodeData) {
        let atlas_index = self.available_ids.pop_front().expect("Out of atlas ids.");

        node.atlas_index = atlas_index;

        self.activated_height_maps
            .push((atlas_index, node.height_map.as_weak()));

        let lod = TerrainConfig::node_position(node.id).0 as usize;
        self.quadtree_update[lod].push(NodeUpdate {
            node_id: node.id,
            atlas_index: node.atlas_index as u32,
        });
    }

    pub(crate) fn deactivate_node(&mut self, node: &mut NodeData) {
        self.available_ids.push_front(node.atlas_index);

        node.atlas_index = Self::INACTIVE_ID;

        let lod = TerrainConfig::node_position(node.id).0 as usize;
        self.quadtree_update[lod].push(NodeUpdate {
            node_id: node.id,
            atlas_index: node.atlas_index as u32,
        });
    }
}

impl ExtractComponent for GpuNodeAtlas {
    type Query = (Write<NodeAtlas>, Read<TerrainConfig>);
    type Filter = Changed<NodeAtlas>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        let (mut node_atlas, config) = item;

        Self {
            node_update_counts: Vec::new(),
            quadtree_update: mem::replace(
                &mut node_atlas.quadtree_update,
                vec![Vec::new(); config.lod_count as usize],
            ),
            activated_height_maps: mem::take(&mut node_atlas.activated_height_maps),
        }
    }
}

pub(crate) fn queue_node_atlas_updates(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
    terrain_data: Res<RenderAssets<TerrainData>>,
    mut terrain_query: Query<(&mut GpuNodeAtlas, &Handle<TerrainData>)>,
) {
    let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

    for (mut gpu_node_atlas, handle) in terrain_query.iter_mut() {
        let gpu_terrain_data = terrain_data.get(handle).unwrap();

        for (index, image) in &gpu_node_atlas.activated_height_maps {
            let image = images.get(&image).unwrap();

            command_encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &image.texture,
                    mip_level: 0,
                    origin: Origin3d { x: 0, y: 0, z: 0 },
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &gpu_terrain_data.height_atlas.texture,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: *index as u32,
                    },
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: gpu_terrain_data.config.texture_size,
                    height: gpu_terrain_data.config.texture_size,
                    depth_or_array_layers: 1,
                },
            );
        }

        let counts = gpu_terrain_data
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
