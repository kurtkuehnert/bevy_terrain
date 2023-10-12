use crate::{
    terrain::{Terrain, TerrainComponents},
    TerrainConfig,
};
use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    pbr::MeshUniform,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::RenderDevice,
        texture::FallbackImage,
        Extract,
    },
};

/// The terrain config data that is available in shaders.
#[derive(Clone, Default, ShaderType)]
struct TerrainConfigUniform {
    lod_count: u32,
    height: f32,
    chunk_size: u32,
    terrain_size: u32,
}

impl From<&TerrainConfig> for TerrainConfigUniform {
    fn from(config: &TerrainConfig) -> Self {
        Self {
            lod_count: config.lod_count,
            height: config.height,
            chunk_size: config.leaf_node_size,
            terrain_size: config.terrain_size,
        }
    }
}

#[derive(AsBindGroup)]
struct TerrainData {
    #[uniform(0, visibility(all))]
    mesh: MeshUniform,
    #[uniform(1, visibility(all))]
    config: TerrainConfigUniform,
    #[sampler(2, visibility(all))]
    #[texture(3, dimension = "2d_array", visibility(all))]
    attachment_1: Option<Handle<Image>>,
    #[texture(4, dimension = "2d_array", visibility(all))]
    attachment_2: Option<Handle<Image>>,
    #[texture(5, dimension = "2d_array", visibility(all))]
    attachment_3: Option<Handle<Image>>,
    #[texture(6, dimension = "2d_array", visibility(all))]
    attachment_4: Option<Handle<Image>>,
    #[texture(7, dimension = "2d_array", visibility(all))]
    attachment_5: Option<Handle<Image>>,
    #[texture(8, dimension = "2d_array", visibility(all))]
    attachment_6: Option<Handle<Image>>,
    #[texture(9, dimension = "2d_array", visibility(all))]
    attachment_7: Option<Handle<Image>>,
    #[texture(10, dimension = "2d_array", visibility(all))]
    attachment_8: Option<Handle<Image>>,
}

pub struct TerrainBindGroup(PreparedBindGroup<()>);

impl TerrainBindGroup {
    fn new(
        config: &TerrainConfig,
        device: &RenderDevice,
        images: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> Self {
        // Todo: pipe this properly
        let mesh = MeshUniform {
            transform: Default::default(),
            previous_transform: Default::default(),
            inverse_transpose_model_a: Default::default(),
            inverse_transpose_model_b: Default::default(),
            flags: 0,
        };

        let attachments = &config.attachments;

        let terrain_data = TerrainData {
            mesh,
            attachment_1: attachments.get(0).map(|a| a.handle.clone()),
            attachment_2: attachments.get(1).map(|a| a.handle.clone()),
            attachment_3: attachments.get(2).map(|a| a.handle.clone()),
            attachment_4: attachments.get(3).map(|a| a.handle.clone()),
            attachment_5: attachments.get(4).map(|a| a.handle.clone()),
            attachment_6: attachments.get(5).map(|a| a.handle.clone()),
            attachment_7: attachments.get(6).map(|a| a.handle.clone()),
            attachment_8: attachments.get(7).map(|a| a.handle.clone()),
            config: config.into(),
        };

        let layout = Self::layout(&device);

        let bind_group = terrain_data
            .as_bind_group(&layout, &device, &images, &fallback_image)
            .ok()
            .unwrap();

        Self(bind_group)
    }

    pub(crate) fn bind_group(&self) -> &BindGroup {
        &self.0.bind_group
    }

    pub(crate) fn layout(device: &RenderDevice) -> BindGroupLayout {
        TerrainData::bind_group_layout(device)
    }
}

pub(crate) fn initialize_terrain_bind_group(
    device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    mut terrain_bind_groups: ResMut<TerrainComponents<TerrainBindGroup>>,
    terrain_query: Extract<Query<(Entity, &TerrainConfig), Added<Terrain>>>,
) {
    for (terrain, config) in terrain_query.iter() {
        let terrain_bind_group = TerrainBindGroup::new(config, &device, &images, &fallback_image);

        terrain_bind_groups.insert(terrain, terrain_bind_group);
    }
}

pub struct SetTerrainBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainBindGroup<I> {
    type Param = SRes<TerrainComponents<TerrainBindGroup>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        terrain_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let terrain_bind_group = terrain_bind_groups
            .into_inner()
            .get(&item.entity())
            .unwrap();

        pass.set_bind_group(I, &terrain_bind_group.bind_group(), &[]);
        RenderCommandResult::Success
    }
}
