use crate::render::{
    layouts::{PATCH_LIST_LAYOUT, TERRAIN_DATA_LAYOUT},
    terrain_data::TerrainData,
};
use bevy::{
    core_pipeline::Opaque3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{
        wireframe::Wireframe, MeshPipeline, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_phase::{
            DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::*,
        renderer::RenderDevice,
        texture::BevyDefault,
    },
};

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct TerrainPipelineKey: u32 {
        const NONE               = 0;
        const WIREFRAME          = (1 << 0);
        const MSAA_RESERVED_BITS = TerrainPipelineKey::MSAA_MASK_BITS << TerrainPipelineKey::MSAA_SHIFT_BITS;
    }
}

impl TerrainPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        TerrainPipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }

    pub fn from_wireframe(wireframe: bool) -> Self {
        TerrainPipelineKey::from_bits(wireframe as u32).unwrap()
    }

    pub fn wireframe(&self) -> bool {
        (self.bits & 1) != 0
    }
}

/// The pipeline used to render the terrain entities.
pub struct TerrainPipeline {
    pub(crate) view_layout: BindGroupLayout,
    pub(crate) mesh_layout: BindGroupLayout,
    pub(crate) terrain_data_layout: BindGroupLayout,
    pub(crate) patch_list_layout: BindGroupLayout,
    pub(crate) shader: Handle<Shader>, // Todo: make fragment shader customizable
}

impl FromWorld for TerrainPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.get_resource::<RenderDevice>().unwrap();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();
        let view_layout = mesh_pipeline.view_layout.clone();
        let mesh_layout = mesh_pipeline.mesh_layout.clone();
        let terrain_data_layout = device.create_bind_group_layout(&TERRAIN_DATA_LAYOUT);
        let patch_list_layout = device.create_bind_group_layout(&PATCH_LIST_LAYOUT);
        let shader = asset_server.load("shaders/terrain.wgsl");

        TerrainPipeline {
            view_layout,
            mesh_layout,
            terrain_data_layout,
            patch_list_layout,
            shader,
        }
    }
}

impl SpecializedPipeline for TerrainPipeline {
    type Key = TerrainPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: None,
            layout: Some(vec![
                self.view_layout.clone(),
                self.mesh_layout.clone(),
                self.terrain_data_layout.clone(),
                self.patch_list_layout.clone(),
            ]),
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: "vertex".into(),
                shader_defs: Vec::new(),
                buffers: Vec::new(),
            },
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: match key.wireframe() {
                    false => PolygonMode::Fill,
                    true => PolygonMode::Line,
                },
                conservative: false,
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }
}

/// The draw function of the terrain. It sets the pipeline and the bind groups and then issues the
/// draw call.
pub(crate) type DrawTerrain = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetTerrainDataBindGroup<2>,
    SetPatchListBindGroup<3>,
    DrawTerrainCommand,
);

pub struct SetTerrainDataBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetTerrainDataBindGroup<I> {
    type Param = (
        SRes<RenderAssets<TerrainData>>,
        SQuery<Read<Handle<TerrainData>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (terrain_data, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let handle = terrain_query.get(item).unwrap();
        let gpu_terrain_data = terrain_data.into_inner().get(handle).unwrap();

        pass.set_bind_group(I, &gpu_terrain_data.terrain_data_bind_group, &[]);

        RenderCommandResult::Success
    }
}

pub struct SetPatchListBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetPatchListBindGroup<I> {
    type Param = (
        SRes<RenderAssets<TerrainData>>,
        SQuery<Read<Handle<TerrainData>>>,
    );

    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (terrain_data, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let handle = terrain_query.get(item).unwrap();
        let gpu_terrain_data = terrain_data.into_inner().get(handle).unwrap();

        pass.set_bind_group(I, &gpu_terrain_data.patch_list_bind_group, &[]);

        RenderCommandResult::Success
    }
}

pub(crate) struct DrawTerrainCommand;

impl EntityRenderCommand for DrawTerrainCommand {
    type Param = (
        SRes<RenderAssets<TerrainData>>,
        SQuery<Read<Handle<TerrainData>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (terrain_data, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let handle = terrain_query.get(item).unwrap();
        let gpu_terrain_data = terrain_data.into_inner().get(handle).unwrap();

        pass.inner()
            .draw_indirect(&gpu_terrain_data.indirect_buffer, 0);

        RenderCommandResult::Success
    }
}

pub(crate) fn extract_terrain(
    mut commands: Commands,
    terrain_query: Query<(Entity, &GlobalTransform), With<Handle<TerrainData>>>,
) {
    for (entity, transform) in terrain_query.iter() {
        let transform = transform.compute_matrix();

        commands.get_or_spawn(entity).insert(MeshUniform {
            flags: 0,
            transform,
            inverse_transpose_model: transform.inverse().transpose(),
        });
    }
}

/// Queses all terrain entities for rendering via the terrain pipeline.
pub(crate) fn queue_terrain(
    terrain_pipeline: Res<TerrainPipeline>,
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedPipelines<TerrainPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut view_query: Query<&mut RenderPhase<Opaque3d>>,
    terrain_query: Query<(Entity, Option<&Wireframe>), With<Handle<TerrainData>>>,
) {
    let draw_function = draw_functions.read().get_id::<DrawTerrain>().unwrap();

    for mut opaque_phase in view_query.iter_mut() {
        for (entity, wireframe) in terrain_query.iter() {
            let key = TerrainPipelineKey::from_msaa_samples(msaa.samples)
                | TerrainPipelineKey::from_wireframe(wireframe.is_some());

            let pipeline = pipelines.specialize(&mut pipeline_cache, &terrain_pipeline, key);

            opaque_phase.add(Opaque3d {
                entity,
                pipeline,
                draw_function,
                distance: f32::MIN, // draw terrain first
            });
        }
    }
}
