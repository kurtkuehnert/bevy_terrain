use crate::{config::TerrainConfigUniform, quadtree::NodeUpdate};
use bevy::{
    prelude::*,
    render::{render_resource::std140::AsStd140, render_resource::*},
};
use std::mem;

pub(crate) const NODE_UPDATE_SIZE: BufferAddress = mem::size_of::<NodeUpdate>() as BufferAddress;
pub(crate) const NODE_SIZE: BufferAddress = mem::size_of::<u32>() as BufferAddress;
pub(crate) const PATCH_SIZE: BufferAddress = 8 * mem::size_of::<u32>() as BufferAddress;
pub(crate) const INDIRECT_BUFFER_SIZE: BufferAddress = 5 * mem::size_of::<u32>() as BufferAddress;
pub(crate) const PARAMETER_BUFFER_SIZE: BufferAddress = 21 * mem::size_of::<u32>() as BufferAddress; // minimum buffer size = 16
pub(crate) const CONFIG_BUFFER_SIZE: BufferAddress =
    mem::size_of::<<TerrainConfigUniform as AsStd140>::Output>() as BufferAddress;
pub(crate) const CULL_DATA_BUFFER_SIZE: BufferAddress = 2 * mem::size_of::<Mat4>() as BufferAddress;

pub(crate) const PREPARE_INDIRECT_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        // config buffer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(CONFIG_BUFFER_SIZE),
            },
            count: None,
        },
        // indirect buffer
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(INDIRECT_BUFFER_SIZE),
            },
            count: None,
        },
        // parameter buffer
        BindGroupLayoutEntry {
            binding: 2,
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
pub(crate) const UPDATE_QUADTREE_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        // quadtree layer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::R16Uint,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // node update buffer
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
pub(crate) const BUILD_NODE_LIST_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        // config buffer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Uint,
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        // parameter buffer
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PARAMETER_BUFFER_SIZE),
            },
            count: None,
        },
        // parent node list
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(NODE_SIZE),
            },
            count: None,
        },
        // child node list
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(NODE_SIZE),
            },
            count: None,
        },
        // final node list
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(NODE_SIZE),
            },
            count: None,
        },
    ],
};
pub(crate) const BUILD_PATCH_LIST_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        // config buffer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(CONFIG_BUFFER_SIZE),
            },
            count: None,
        },
        // quadtree
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Uint,
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        // parameter buffer
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PARAMETER_BUFFER_SIZE),
            },
            count: None,
        },
        // node list
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(NODE_SIZE),
            },
            count: None,
        },
        // patch list
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PATCH_SIZE),
            },
            count: None,
        },
        // lod map
        BindGroupLayoutEntry {
            binding: 5,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Uint,
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
    ],
};
pub(crate) const BUILD_CHUNK_MAPS_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        // config buffer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(CONFIG_BUFFER_SIZE),
            },
            count: None,
        },
        // quadtree
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Uint,
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        // parameter buffer
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PARAMETER_BUFFER_SIZE),
            },
            count: None,
        },
        // node list
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(NODE_SIZE),
            },
            count: None,
        },
        // lod map
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::R8Uint,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // atlas map
        BindGroupLayoutEntry {
            binding: 5,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::R16Uint,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
    ],
};
pub(crate) const CULL_DATA_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    // cull data
    label: None,
    entries: &[BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(CULL_DATA_BUFFER_SIZE),
        },
        count: None,
    }],
};
pub(crate) const PATCH_LIST_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::VERTEX,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(PATCH_SIZE),
        },
        count: None,
    }],
};
