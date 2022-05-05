use crate::{render::gpu_node_atlas::NodeAttachmentConfig, TerrainConfig};
use bevy::render::render_resource::*;

pub fn add_height_attachment_config(config: &mut TerrainConfig, texture_size: u32) {
    let texture_descriptor = TextureDescriptor {
        label: None,
        size: Extent3d {
            width: texture_size,
            height: texture_size,
            depth_or_array_layers: config.node_atlas_size as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R16Unorm,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
    };

    let view_descriptor = TextureViewDescriptor {
        label: None,
        format: None,
        dimension: Some(TextureViewDimension::D2Array),
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    };

    let sampler_descriptor = SamplerDescriptor {
        label: None,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        compare: None,
        anisotropy_clamp: None,
        border_color: None,
    };

    config.add_node_attachment_config(
        "height_map".into(),
        NodeAttachmentConfig::Texture {
            view_binding: 2,
            sampler_binding: 3,
            texture_size,
            texture_descriptor,
            view_descriptor,
            sampler_descriptor,
        },
    );
}

pub fn add_albedo_attachment_config(config: &mut TerrainConfig, texture_size: u32) {
    let texture_descriptor = TextureDescriptor {
        label: None,
        size: Extent3d {
            width: texture_size,
            height: texture_size,
            depth_or_array_layers: config.node_atlas_size as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
    };

    let view_descriptor = TextureViewDescriptor {
        label: None,
        format: None,
        dimension: Some(TextureViewDimension::D2Array),
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    };

    let sampler_descriptor = SamplerDescriptor {
        label: None,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        compare: None,
        anisotropy_clamp: None,
        border_color: None,
    };

    config.add_node_attachment_config(
        "albedo_map".into(),
        NodeAttachmentConfig::Texture {
            view_binding: 4,
            sampler_binding: 5,
            texture_size,
            texture_descriptor,
            view_descriptor,
            sampler_descriptor,
        },
    );
}
