use crate::{
    config::TerrainConfig,
    node_atlas::NodeAtlas,
    quadtree::NodeData,
    render::{InitTerrain, PersistentComponent},
};
use bevy::{
    prelude::*,
    render::{render_resource::*, RenderWorld},
    utils::HashMap,
};
use std::mem;

pub enum NodeAttachment {
    Buffer(Buffer),
    Texture {
        texture: Texture,
        view: TextureView,
        sampler: Sampler,
    },
}

pub struct GpuNodeAtlas {
    pub(crate) attachment_order: Vec<String>,
    pub(crate) atlas_attachments: HashMap<String, NodeAttachment>,
    pub(crate) activated_nodes: Vec<(u16, NodeData)>, // make generic on NodeData
}

impl GpuNodeAtlas {
    fn new() -> Self {
        Self {
            attachment_order: vec!["heightmap".into()],
            atlas_attachments: Default::default(),
            activated_nodes: vec![],
        }
    }
}

/// Runs in prepare.
pub(crate) fn init_node_atlas(
    mut gpu_node_atlases: ResMut<PersistentComponent<GpuNodeAtlas>>,
    terrain_query: Query<Entity, With<InitTerrain>>,
) {
    for entity in terrain_query.iter() {
        info!("initializing gpu node atlas");

        gpu_node_atlases.insert(entity, GpuNodeAtlas::new());
    }
}

pub(crate) fn extract_node_atlas(
    mut render_world: ResMut<RenderWorld>,
    mut terrain_query: Query<(Entity, &mut NodeAtlas), With<TerrainConfig>>,
) {
    let mut gpu_node_atlases = render_world.resource_mut::<PersistentComponent<GpuNodeAtlas>>();

    for (entity, mut node_atlas) in terrain_query.iter_mut() {
        let gpu_node_atlas = match gpu_node_atlases.get_mut(&entity) {
            Some(gpu_node_atlas) => gpu_node_atlas,
            None => continue,
        };

        gpu_node_atlas.activated_nodes = mem::take(&mut node_atlas.activated_nodes);
    }
}
