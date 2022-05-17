use crate::{
    config::TerrainConfigUniform,
    quadtree::{NodeActivation, NodeDeactivation},
    render::culling::CullingData,
};
use bevy::{
    core::{Pod, Zeroable},
    render::{render_resource::std140::AsStd140, render_resource::*},
};
use std::mem;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
struct Patch {
    x: u32,
    y: u32,
    size: u32,
    stitch: u32,
}

pub(crate) const NODE_ACTIVATION_SIZE: BufferAddress =
    mem::size_of::<NodeActivation>() as BufferAddress;
pub(crate) const NODE_DEACTIVATION_SIZE: BufferAddress =
    mem::size_of::<NodeDeactivation>() as BufferAddress;
pub(crate) const PATCH_SIZE: BufferAddress = mem::size_of::<Patch>() as BufferAddress;
pub(crate) const INDIRECT_BUFFER_SIZE: BufferAddress = 5 * mem::size_of::<u32>() as BufferAddress;
pub(crate) const PARAMETER_BUFFER_SIZE: BufferAddress = 2 * mem::size_of::<u32>() as BufferAddress; // minimum buffer size = 16
pub(crate) const CONFIG_BUFFER_SIZE: BufferAddress =
    mem::size_of::<<TerrainConfigUniform as AsStd140>::Output>() as BufferAddress;
pub(crate) const CULL_DATA_BUFFER_SIZE: BufferAddress =
    mem::size_of::<CullingData>() as BufferAddress;

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
        // quadtree
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
        // node activations
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(NODE_ACTIVATION_SIZE),
            },
            count: None,
        },
        // node deactivations
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(NODE_DEACTIVATION_SIZE),
            },
            count: None,
        },
    ],
};
pub(crate) const TESSELATION_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
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
        // parent patch list
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PATCH_SIZE),
            },
            count: None,
        },
        // child patch list
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(PATCH_SIZE),
            },
            count: None,
        },
        // final patch list
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
    // patch list
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
