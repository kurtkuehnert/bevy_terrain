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
pub(crate) struct TerrainConfigUniform {
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
pub(crate) struct TerrainData {
    #[uniform(0)]
    mesh: MeshUniform,
    #[uniform(1)]
    config: TerrainConfigUniform,
    #[sampler(2, visibility(all))]
    #[texture(3, dimension = "2d_array")]
    attachment_1: Option<Handle<Image>>,
    #[texture(4, dimension = "2d_array")]
    attachment_2: Option<Handle<Image>>,
    #[texture(5, dimension = "2d_array")]
    attachment_3: Option<Handle<Image>>,
    #[texture(6, dimension = "2d_array")]
    attachment_4: Option<Handle<Image>>,
    #[texture(7, dimension = "2d_array")]
    attachment_5: Option<Handle<Image>>,
    #[texture(8, dimension = "2d_array")]
    attachment_6: Option<Handle<Image>>,
    #[texture(9, dimension = "2d_array")]
    attachment_7: Option<Handle<Image>>,
    #[texture(10, dimension = "2d_array")]
    attachment_8: Option<Handle<Image>>,
}

impl TerrainData {
    pub(crate) fn new(config: &TerrainConfig) -> Self {
        // Todo: pipe this properly
        let mesh = MeshUniform {
            transform: Default::default(),
            previous_transform: Default::default(),
            inverse_transpose_model: Default::default(),
            flags: 0,
        };

        let attachments = &config.attachments;
        let attachment_1 = attachments.get(0).map(|a| a.handle.clone());
        let attachment_2 = attachments.get(1).map(|a| a.handle.clone());
        let attachment_3 = attachments.get(2).map(|a| a.handle.clone());
        let attachment_4 = attachments.get(3).map(|a| a.handle.clone());
        let attachment_5 = attachments.get(4).map(|a| a.handle.clone());
        let attachment_6 = attachments.get(5).map(|a| a.handle.clone());
        let attachment_7 = attachments.get(6).map(|a| a.handle.clone());
        let attachment_8 = attachments.get(7).map(|a| a.handle.clone());

        let config = config.into();

        Self {
            mesh,
            config,
            attachment_1,
            attachment_2,
            attachment_3,
            attachment_4,
            attachment_5,
            attachment_6,
            attachment_7,
            attachment_8,
        }
    }
}

pub struct TerrainDataBindGroup(PreparedBindGroup<()>);

impl TerrainDataBindGroup {
    pub(crate) fn bind_group(&self) -> &BindGroup {
        &self.0.bind_group
    }
}

pub(crate) fn initialize_terrain_data(
    device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    mut terrain_data: ResMut<TerrainComponents<TerrainDataBindGroup>>,
    terrain_query: Extract<Query<(Entity, &TerrainConfig), Added<Terrain>>>,
) {
    for (terrain, config) in terrain_query.iter() {
        let layout = TerrainData::bind_group_layout(&device);
        let data = TerrainData::new(config);
        let data = TerrainDataBindGroup(
            data.as_bind_group(&layout, &device, &images, &fallback_image)
                .ok()
                .unwrap(),
        );

        terrain_data.insert(terrain, data);
    }
}

pub struct SetTerrainBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetTerrainBindGroup<I> {
    type Param = SRes<TerrainComponents<TerrainDataBindGroup>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        _: ROQueryItem<'w, Self::ItemWorldQuery>,
        terrain_data: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let data = terrain_data.into_inner().get(&item.entity()).unwrap();
        pass.set_bind_group(I, &data.bind_group(), &[]);
        RenderCommandResult::Success
    }
}
