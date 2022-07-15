use crate::{quadtree::NodeUpdate, render::culling::CullingData, terrain::TerrainConfigUniform};
use bevy::render::render_resource::*;
use std::mem;

pub(crate) const TERRAIN_VIEW_CONFIG_SIZE: BufferAddress = 4 * 48;

pub(crate) const TILE_SIZE: BufferAddress = 6 * 4;
pub(crate) const INDIRECT_BUFFER_SIZE: BufferAddress = 5 * 4;
pub(crate) const PARAMETER_BUFFER_SIZE: BufferAddress = 6 * 4;
pub(crate) const NODE_UPDATE_SIZE: BufferAddress = mem::size_of::<NodeUpdate>() as BufferAddress;
pub(crate) const CONFIG_BUFFER_SIZE: BufferAddress =
    mem::size_of::<TerrainConfigUniform>() as BufferAddress;
pub(crate) const CULL_DATA_BUFFER_SIZE: BufferAddress =
    mem::size_of::<CullingData>() as BufferAddress;

pub(crate) const UPDATE_QUADTREE_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: Some("update_quadtree_layout"),
    entries: &[
        // quadtree array texture
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::ReadWrite,
                format: TextureFormat::Rgba8Uint,
                view_dimension: TextureViewDimension::D2Array,
            },
            count: None,
        },
        // node updates buffer
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(NODE_UPDATE_SIZE),
            },
            count: None,
        },
    ],
};

pub(crate) const PREPARE_INDIRECT_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: Some("prepare_indirect_layout"),
    entries: &[
        // indirect buffer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(INDIRECT_BUFFER_SIZE),
            },
            count: None,
        },
    ],
};

pub(crate) const CULL_DATA_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: Some("cull data_layout"),
    entries: &[
        // cull data
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(CULL_DATA_BUFFER_SIZE),
            },
            count: None,
        },
    ],
};

pub(crate) const TESSELLATION_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: Some("tessellation_layout"),
    entries: &[
        // view config buffer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // quadtree array texture
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Uint,
                view_dimension: TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        // final tile buffer
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(32 + TILE_SIZE),
            },
            count: None,
        },
        // temporary tile buffer
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(32 + TILE_SIZE),
            },
            count: None,
        },
        // parameter buffer
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PARAMETER_BUFFER_SIZE),
            },
            count: None,
        },
    ],
};

pub(crate) const TERRAIN_VIEW_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: Some("terrain_view_layout"),
    entries: &[
        // view config buffer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // quadtree array texture
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Uint,
                view_dimension: TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        // tile buffer
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(32 + TILE_SIZE),
            },
            count: None,
        },
    ],
};
