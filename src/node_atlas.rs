use crate::config::TerrainConfig;
use crate::{quadtree::NodeData, TerrainComputePipeline, TerrainData};
use bevy::core::{Pod, Zeroable};
use bevy::render::render_resource::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource,
};
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
pub(crate) struct NodeUpdate {
    pub(crate) node_id: u32,
    pub(crate) atlas_index: u32, // u16 not supported by std 140
}

#[derive(Component)]
pub struct GpuQuadtreeUpdate(pub(crate) Vec<(u32, BindGroup)>);

/// Maps the assets to the corresponding active nodes and tracks the node updates.
#[derive(Component)]
pub struct NodeAtlas {
    pub(crate) available_ids: Vec<u16>,
    pub(crate) quadtree_update: Vec<NodeUpdate>,
    activated_height_maps: Vec<(u16, Handle<Image>)>,
}

impl NodeAtlas {
    // pub(crate) const NONEXISTING_ID: u16 = u16::MAX;
    pub(crate) const INACTIVE_ID: u16 = u16::MAX - 1;

    pub fn new(atlas_size: u16) -> Self {
        Self {
            available_ids: (0..atlas_size).collect(),
            quadtree_update: Default::default(),
            activated_height_maps: Default::default(),
        }
    }

    pub(crate) fn activate_node(&mut self, node: &mut NodeData) {
        let atlas_index = self.available_ids.pop().expect("Out of atlas ids.");

        node.atlas_index = atlas_index;

        self.quadtree_update.push(NodeUpdate {
            node_id: node.id,
            atlas_index: node.atlas_index as u32,
        });

        self.activated_height_maps
            .push((atlas_index, node.height_map.as_weak()));
    }

    pub(crate) fn deactivate_node(&mut self, node: &mut NodeData) {
        self.available_ids.push(node.atlas_index);

        node.atlas_index = Self::INACTIVE_ID;

        self.quadtree_update.push(NodeUpdate {
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
            quadtree_update: mem::take(&mut item.quadtree_update),
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

pub(crate) fn queue_quadtree_update(
    mut commands: Commands,
    mut device: ResMut<RenderDevice>,
    mut queue: ResMut<RenderQueue>,
    pipeline: Res<TerrainComputePipeline>,
    terrain_data: ResMut<RenderAssets<TerrainData>>,
    terrain_query: Query<(Entity, &NodeAtlas, &Handle<TerrainData>)>,
) {
    let terrain_data = terrain_data.into_inner();

    for (entity, node_atlas, handle) in terrain_query.iter() {
        let gpu_terrain_data = terrain_data.get_mut(handle).unwrap();
        let quadtree_data = &mut gpu_terrain_data.quadtree_data;

        // insert the node update into the buffer corresponding to its lod
        node_atlas.quadtree_update.iter().for_each(|&data| {
            let lod = TerrainConfig::node_position(data.node_id).0 as usize;
            quadtree_data[lod].0.push(data);
        });

        // create the bind groups for each lod
        let data = quadtree_data
            .iter_mut()
            .map(|(buffer, view)| {
                buffer.write_buffer(&mut device, &mut queue);

                let count = buffer.len() as u32;

                let bind_group = device.create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.update_quadtree_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: buffer.buffer().unwrap().as_entire_binding(),
                        },
                    ],
                });

                buffer.clear(); // reset buffer for next frame

                (count, bind_group)
            })
            .collect();

        commands.entity(entity).insert(GpuQuadtreeUpdate(data));
    }
}
