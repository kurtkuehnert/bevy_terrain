use crate::pipeline::{prepare_terrain, queue_terrain, DrawTerrain, TerrainData, TerrainPipeline};
use bevy::{
    core_pipeline::Opaque3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_phase::{
            AddRenderCommand, EntityRenderCommand, RenderCommandResult, TrackedRenderPass,
        },
        render_resource::{
            std140::{AsStd140, Std140},
            *,
        },
        renderer::RenderDevice,
        RenderApp, RenderStage,
    },
};

pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TerrainMaterial>()
            .add_plugin(RenderAssetPlugin::<TerrainMaterial>::default())
            .add_plugin(ExtractComponentPlugin::<Handle<TerrainMaterial>>::default())
            .add_plugin(ExtractComponentPlugin::<TerrainData>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain>()
            .init_resource::<TerrainPipeline>()
            .init_resource::<SpecializedPipelines<TerrainPipeline>>()
            .add_system_to_stage(RenderStage::Prepare, prepare_terrain)
            .add_system_to_stage(RenderStage::Queue, queue_terrain);
    }
}

/// The GPU representation of the uniform data of a [`TerrainMaterial`].
#[derive(Clone, Default, AsStd140)]
struct TerrainMaterialUniformData {
    height: f32,
}

/// The GPU representation of a [`TerrainMaterial`].
#[derive(Debug, Clone)]
pub struct GpuTerrainMaterial {
    /// A buffer containing the [`TerrainMaterialUniformData`] of the material.
    _buffer: Buffer,
    /// The bind group specifying how the [`TerrainMaterialUniformData`] and
    /// all the textures of the material are bound.
    bind_group: BindGroup,
}

/// The material of the terrain, specifying its textures and parameters.
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7cc8ded7-03f9-4ed5-8379-a256f94613ff"]
pub struct TerrainMaterial {
    pub height_texture: Handle<Image>,
    pub height: f32,
}

impl TerrainMaterial {
    /// Returns this material's [`BindGroup`]. This should match the layout returned by [`SpecializedMaterial::bind_group_layout`].
    #[inline]
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &material.bind_group
    }

    /// Returns this material's [`BindGroupLayout`]. This should match the [`BindGroup`] returned by [`SpecializedMaterial::bind_group`].
    pub(crate) fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // Uniform Data
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            TerrainMaterialUniformData::std140_size_static() as u64,
                        ),
                    },
                    count: None,
                },
                // Height Texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Height Texture Sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("terrain_material_layout"),
        })
    }

    /// The dynamic uniform indices to set for the given `material`'s [`BindGroup`].
    /// Defaults to an empty array / no dynamic uniform indices.
    #[inline]
    fn dynamic_uniform_indices(_material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        &[]
    }
}

impl RenderAsset for TerrainMaterial {
    type ExtractedAsset = TerrainMaterial;
    type PreparedAsset = GpuTerrainMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<TerrainPipeline>,
        SRes<RenderAssets<Image>>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (render_device, terrain_pipeline, gpu_images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let (height_texture_view, height_texture_sampler) =
            match gpu_images.get(&material.height_texture) {
                Some(gpu_image) => (&gpu_image.texture_view, &gpu_image.sampler),
                None => return Err(PrepareAssetError::RetryNextUpdate(material)),
            };

        let uniform_data = TerrainMaterialUniformData {
            height: material.height,
        };

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("pbr_standard_material_uniform_buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: uniform_data.as_std140().as_bytes(),
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(height_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(height_texture_sampler),
                },
            ],
            label: Some("terrain_material_bind_group"),
            layout: &terrain_pipeline.material_layout,
        });

        Ok(GpuTerrainMaterial {
            _buffer: buffer,
            bind_group,
        })
    }
}

pub struct SetTerrainMaterialBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetTerrainMaterialBindGroup<I> {
    type Param = (
        SRes<RenderAssets<TerrainMaterial>>,
        SQuery<Read<Handle<TerrainMaterial>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (materials, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material = terrain_query.get(item).unwrap();
        let material = match materials.into_inner().get(material) {
            Some(material) => material,
            None => return RenderCommandResult::Failure,
        };

        pass.set_bind_group(
            I,
            TerrainMaterial::bind_group(material),
            TerrainMaterial::dynamic_uniform_indices(material),
        );
        RenderCommandResult::Success
    }
}
