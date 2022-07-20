use bevy::{prelude::*, render::render_resource::*};

pub(crate) mod gpu_node_atlas;
pub(crate) mod gpu_quadtree;
pub(crate) mod node_atlas;
pub(crate) mod quadtree;

// Todo: may be swap to u64 for giant terrains
// Todo: consider 3 bit face data, for cube sphere
/// A globally unique identifier of a node.
/// lod |  x |  y
///   4 | 14 | 14
pub(crate) type NodeId = u32;
pub(crate) const INVALID_NODE_ID: NodeId = NodeId::MAX;
pub(crate) const INVALID_LOD: u16 = u16::MAX;

/// Identifier of an active node (and its attachments) inside the node atlas.
pub type AtlasIndex = u16;
pub(crate) const INVALID_ATLAS_INDEX: AtlasIndex = AtlasIndex::MAX;

pub type AttachmentIndex = usize;

#[inline]
pub(crate) fn calc_node_id(lod: u32, x: u32, y: u32) -> NodeId {
    (lod & 0xF) << 28 | (x & 0x3FFF) << 14 | y & 0x3FFF
}

/// The global coordinate of a node.
pub struct NodeCoordinate {
    pub lod: u32,
    pub x: u32,
    pub y: u32,
}

impl From<NodeId> for NodeCoordinate {
    /// Determines the coordinate of the node based on its id.
    #[inline]
    fn from(id: NodeId) -> Self {
        Self {
            lod: (id >> 28) & 0xF,
            x: (id >> 14) & 0x3FFF,
            y: id & 0x3FFF,
        }
    }
}

/// Configures an attachment of a [`NodeAtlas`](crate::data_structures::node_atlas::NodeAtlas).
#[derive(Clone)]
pub struct AtlasAttachment {
    pub(crate) handle: Handle<Image>,
    pub(crate) name: &'static str,
    pub(crate) texture_size: u32,
    pub(crate) border_size: u32,
    pub(crate) format: TextureFormat,
}
