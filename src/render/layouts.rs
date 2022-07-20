use crate::{
    render::culling::CullingData, terrain::TerrainConfigUniform,
    terrain_view::TerrainViewConfigUniform,
};
use bevy::render::render_resource::*;
use std::mem;

pub(crate) const TERRAIN_CONFIG_SIZE: BufferAddress =
    mem::size_of::<TerrainConfigUniform>() as BufferAddress;
pub(crate) const TERRAIN_VIEW_CONFIG_SIZE: BufferAddress =
    mem::size_of::<TerrainViewConfigUniform>() as BufferAddress;
pub(crate) const CULL_DATA_BUFFER_SIZE: BufferAddress =
    mem::size_of::<CullingData>() as BufferAddress;
pub(crate) const TILE_SIZE: BufferAddress = 6 * 4;
pub(crate) const INDIRECT_BUFFER_SIZE: BufferAddress = 5 * 4;
pub(crate) const PARAMETER_BUFFER_SIZE: BufferAddress = 6 * 4;

pub(crate) const PREPARE_INDIRECT_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
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
    label: None,
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
    label: None,
    entries: &[
        // view config
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
        // quadtree
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
        // final tiles
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
        // temporary tiles
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
        // parameters
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
    label: None,
    entries: &[
        // view config
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
        // quadtree
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
        // tiles
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
