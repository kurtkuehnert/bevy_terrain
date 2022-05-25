use crate::attachment::{AtlasAttachment, AttachmentIndex};
use bevy::render::render_graph;
use bevy::render::render_graph::{NodeRunError, RenderGraphContext};
use bevy::render::renderer::RenderContext;
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        RenderWorld,
    },
    utils::HashMap,
};

pub struct ProcessingComponent {
    pub texture_attachments: HashMap<AttachmentIndex, Texture>,
}

pub struct ImportTexture {
    image: Handle<Image>,
    position: UVec2,
    size: u32,
}

pub struct TerrainProcessingNode {}

impl render_graph::Node for TerrainProcessingNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        todo!()
    }
}
