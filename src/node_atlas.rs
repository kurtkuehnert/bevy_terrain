use crate::{quadtree::NodeData, quadtree_update::NodeUpdate, TerrainData};
use bevy::{
    ecs::{query::QueryItem, system::lifetimeless::Write},
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_component::ExtractComponent,
        render_resource::{
            CommandEncoderDescriptor, Extent3d, ImageCopyTexture, Origin3d, TextureAspect,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use std::mem;

pub struct NodeAtlasUpdate {}

/// Maps the assets to the corresponding active nodes and tracks the node updates.
#[derive(Component)]
pub struct NodeAtlas {
    pub(crate) available_ids: Vec<u16>,
    activated_height_maps: Vec<(u16, Handle<Image>)>,
}

impl NodeAtlas {
    // pub(crate) const NONEXISTING_ID: u16 = u16::MAX;
    pub(crate) const INACTIVE_ID: u16 = u16::MAX - 1;

    pub fn new(atlas_size: u16) -> Self {
        Self {
            available_ids: (0..atlas_size).collect(),
            activated_height_maps: vec![],
        }
    }

    pub(crate) fn activate_node(&mut self, node: &mut NodeData, updates: &mut Vec<NodeUpdate>) {
        let atlas_index = self.available_ids.pop().expect("Out of atlas ids.");

        node.atlas_index = atlas_index;

        updates.push(NodeUpdate {
            node_id: node.id,
            atlas_index: node.atlas_index as u32,
        });

        self.activated_height_maps
            .push((atlas_index, node.height_map.as_weak()));
    }

    pub(crate) fn deactivate_node(&mut self, node: &mut NodeData, updates: &mut Vec<NodeUpdate>) {
        self.available_ids.push(node.atlas_index);

        node.atlas_index = Self::INACTIVE_ID;

        updates.push(NodeUpdate {
            node_id: node.id,
            atlas_index: node.atlas_index as u32,
        });
    }
}

impl ExtractComponent for NodeAtlas {
    type Query = Write<NodeAtlas>;
    type Filter = Changed<NodeAtlas>;

    fn extract_component(mut item: QueryItem<Self::Query>) -> Self {
        Self {
            available_ids: Vec::new(),
            activated_height_maps: mem::take(&mut item.activated_height_maps),
        }
    }
}

pub(crate) fn queue_atlas_updates(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
    terrain_data: Res<RenderAssets<TerrainData>>,
    terrain_query: Query<(&NodeAtlas, &Handle<TerrainData>)>,
) {
    let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

    for (atlas, handle) in terrain_query.iter() {
        let gpu_terrain_data = terrain_data.get(handle).unwrap();

        for (index, image) in &atlas.activated_height_maps {
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
    }

    queue.submit(vec![command_encoder.finish()]);
}
