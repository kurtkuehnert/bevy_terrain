use crate::{
    render::{
        shaders::DEFAULT_SHADER,
        terrain_data::{terrain_bind_group_layout, SetTerrainBindGroup},
        terrain_view_data::{DrawTerrainCommand, SetTerrainViewBindGroup},
        TERRAIN_VIEW_LAYOUT,
    },
    DebugTerrain, Terrain,
};
use bevy::{
    core_pipeline::core_3d::Opaque3d,
    pbr::{MeshPipeline, RenderMaterials, SetMaterialBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        render_phase::{AddRenderCommand, DrawFunctions, RenderPhase, SetItemPipeline},
        render_resource::*,
        renderer::RenderDevice,
        texture::BevyDefault,
        RenderApp, RenderSet,
    },
};
use std::{hash::Hash, marker::PhantomData};
use bevy::pbr::{MAX_CASCADES_PER_LIGHT, MAX_DIRECTIONAL_LIGHTS};

/// Configures the default terrain pipeline.
#[derive(Resource)]
pub struct TerrainPipelineConfig {
    /// The number of terrain attachments.
    pub attachment_count: usize,
}

pub struct TerrainPipelineKey<M: Material> {
    pub flags: TerrainPipelineFlags,
    pub bind_group_data: M::Data,
}

impl<M: Material> Eq for TerrainPipelineKey<M> where M::Data: PartialEq {}

impl<M: Material> PartialEq for TerrainPipelineKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.flags == other.flags && self.bind_group_data == other.bind_group_data
    }
}

impl<M: Material> Clone for TerrainPipelineKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            flags: self.flags,
            bind_group_data: self.bind_group_data.clone(),
        }
    }
}

impl<M: Material> Hash for TerrainPipelineKey<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.flags.hash(state);
        self.bind_group_data.hash(state);
    }
}

bitflags::bitflags! {
#[repr(transparent)]
pub struct TerrainPipelineFlags: u32 {
    const NONE               = 0;
    const WIREFRAME          = (1 <<  0);
    const SHOW_TILES         = (1 <<  1);
    const SHOW_LOD           = (1 <<  2);
    const SHOW_UV            = (1 <<  3);
    const SHOW_NODES         = (1 <<  4);
    const SHOW_MINMAX_ERROR  = (1 <<  5);
    const MINMAX             = (1 <<  6);
    const MESH_MORPH         = (1 <<  7);
    const ALBEDO             = (1 <<  8);
    const BRIGHT             = (1 <<  9);
    const LIGHTING           = (1 << 10);
    const SAMPLE_GRAD        = (1 << 11);
    const TEST1              = (1 << 12);
    const TEST2              = (1 << 13);
    const TEST3              = (1 << 14);

    const MSAA_RESERVED_BITS = TerrainPipelineFlags::MSAA_MASK_BITS << TerrainPipelineFlags::MSAA_SHIFT_BITS;
}
}

impl TerrainPipelineFlags {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        TerrainPipelineFlags::from_bits(msaa_bits).unwrap()
    }

    pub fn from_debug(debug: &DebugTerrain) -> Self {
        let mut key = TerrainPipelineFlags::NONE;

        if debug.wireframe {
            key |= TerrainPipelineFlags::WIREFRAME;
        }
        if debug.show_tiles {
            key |= TerrainPipelineFlags::SHOW_TILES;
        }
        if debug.show_lod {
            key |= TerrainPipelineFlags::SHOW_LOD;
        }
        if debug.show_uv {
            key |= TerrainPipelineFlags::SHOW_UV;
        }
        if debug.show_minmax_error {
            key |= TerrainPipelineFlags::SHOW_MINMAX_ERROR;
        }
        if debug.minmax {
            key |= TerrainPipelineFlags::MINMAX;
        }
        if debug.show_nodes {
            key |= TerrainPipelineFlags::SHOW_NODES;
        }
        if debug.mesh_morph {
            key |= TerrainPipelineFlags::MESH_MORPH;
        }
        if debug.albedo {
            key |= TerrainPipelineFlags::ALBEDO;
        }
        if debug.bright {
            key |= TerrainPipelineFlags::BRIGHT;
        }
        if debug.lighting {
            key |= TerrainPipelineFlags::LIGHTING;
        }
        if debug.sample_grad {
            key |= TerrainPipelineFlags::SAMPLE_GRAD;
        }
        if debug.test1 {
            key |= TerrainPipelineFlags::TEST1;
        }
        if debug.test2 {
            key |= TerrainPipelineFlags::TEST2;
        }
        if debug.test3 {
            key |= TerrainPipelineFlags::TEST3;
        }

        key
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }

    pub fn polygon_mode(&self) -> PolygonMode {
        match (self.bits & TerrainPipelineFlags::WIREFRAME.bits) != 0 {
            true => PolygonMode::Line,
            false => PolygonMode::Fill,
        }
    }

    pub fn shader_defs(&self) -> Vec<ShaderDefVal> {
        let mut shader_defs = Vec::new();

        if (self.bits & TerrainPipelineFlags::SHOW_TILES.bits) != 0 {
            shader_defs.push("SHOW_TILES".into());
        }
        if (self.bits & TerrainPipelineFlags::SHOW_LOD.bits) != 0 {
            shader_defs.push("SHOW_LOD".into());
        }
        if (self.bits & TerrainPipelineFlags::SHOW_UV.bits) != 0 {
            shader_defs.push("SHOW_UV".into());
        }
        if (self.bits & TerrainPipelineFlags::SHOW_NODES.bits) != 0 {
            shader_defs.push("SHOW_NODES".into());
        }
        if (self.bits & TerrainPipelineFlags::SHOW_MINMAX_ERROR.bits) != 0 {
            shader_defs.push("SHOW_MINMAX_ERROR".into());
        }
        if (self.bits & TerrainPipelineFlags::MINMAX.bits) != 0 {
            shader_defs.push("MINMAX".into());
        }
        if (self.bits & TerrainPipelineFlags::MESH_MORPH.bits) != 0 {
            shader_defs.push("MESH_MORPH".into());
        }
        if (self.bits & TerrainPipelineFlags::ALBEDO.bits) != 0 {
            shader_defs.push("ALBEDO".into());
        }
        if (self.bits & TerrainPipelineFlags::BRIGHT.bits) != 0 {
            shader_defs.push("BRIGHT".into());
        }
        if (self.bits & TerrainPipelineFlags::LIGHTING.bits) != 0 {
            shader_defs.push("LIGHTING".into());
        }
        if (self.bits & TerrainPipelineFlags::SAMPLE_GRAD.bits) != 0 {
            shader_defs.push("SAMPLE_GRAD".into());
        }
        if (self.bits & TerrainPipelineFlags::TEST1.bits) != 0 {
            shader_defs.push("TEST1".into());
        }
        if (self.bits & TerrainPipelineFlags::TEST2.bits) != 0 {
            shader_defs.push("TEST2".into());
        }
        if (self.bits & TerrainPipelineFlags::TEST3.bits) != 0 {
            shader_defs.push("TEST3".into());
        }

        shader_defs
    }
}

/// The pipeline used to render the terrain entities.
#[derive(Resource)]
pub struct TerrainRenderPipeline<M: Material> {
    pub(crate) view_layout: BindGroupLayout,
    pub(crate) view_layout_multisampled: BindGroupLayout,
    pub(crate) terrain_layout: BindGroupLayout,
    pub(crate) terrain_view_layout: BindGroupLayout,
    pub(crate) material_layout: BindGroupLayout,
    pub vertex_shader: Handle<Shader>,
    pub fragment_shader: Handle<Shader>,
    marker: PhantomData<M>,
}

impl<M: Material> FromWorld for TerrainRenderPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();
        let mesh_pipeline = world.resource::<MeshPipeline>();
        let config = world.resource::<TerrainPipelineConfig>();

        let view_layout = mesh_pipeline.view_layout.clone();
        let view_layout_multisampled = mesh_pipeline.view_layout_multisampled.clone();
        let terrain_layout = terrain_bind_group_layout(device, config.attachment_count);
        let terrain_view_layout = device.create_bind_group_layout(&TERRAIN_VIEW_LAYOUT);
        let material_layout = M::bind_group_layout(device);

        let vertex_shader = match M::vertex_shader() {
            ShaderRef::Default => DEFAULT_SHADER.typed(),
            ShaderRef::Handle(handle) => handle,
            ShaderRef::Path(path) => asset_server.load(path),
        };

        let fragment_shader = match M::fragment_shader() {
            ShaderRef::Default => DEFAULT_SHADER.typed(),
            ShaderRef::Handle(handle) => handle,
            ShaderRef::Path(path) => asset_server.load(path),
        };

        Self {
            view_layout,
            view_layout_multisampled,
            terrain_layout,
            terrain_view_layout,
            material_layout,
            vertex_shader,
            fragment_shader,
            marker: PhantomData,
        }
    }
}

impl<M: Material> SpecializedRenderPipeline for TerrainRenderPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = TerrainPipelineKey<M>;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = key.flags.shader_defs();

        shader_defs.push(ShaderDefVal::UInt("MAX_DIRECTIONAL_LIGHTS".to_string(), MAX_DIRECTIONAL_LIGHTS as u32));
        shader_defs.push(ShaderDefVal::UInt("MAX_CASCADES_PER_LIGHT".to_string(), MAX_CASCADES_PER_LIGHT as u32));

        let mut bind_group_layout = match key.flags.msaa_samples() {
            1 => vec![self.view_layout.clone()],
            _ => {
                shader_defs.push("MULTISAMPLED".into());
                vec![self.view_layout_multisampled.clone()]
            }
        };

        bind_group_layout.push(self.terrain_view_layout.clone());
        bind_group_layout.push(self.terrain_layout.clone()); // Todo: do this properly for multiple terrains
        bind_group_layout.push(self.material_layout.clone());

        RenderPipelineDescriptor {
            label: None,
            layout: bind_group_layout,
            push_constant_ranges: default(),
            vertex: VertexState {
                shader: self.vertex_shader.clone(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: Vec::new(),
            },
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: key.flags.polygon_mode(),
                conservative: false,
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
            },
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
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
                count: key.flags.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }
}

/// The draw function of the terrain. It sets the pipeline and the bind groups and then issues the
/// draw call.
pub(crate) type DrawTerrain<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetTerrainViewBindGroup<1>,
    SetTerrainBindGroup<2>,
    SetMaterialBindGroup<M, 3>,
    DrawTerrainCommand,
);

/// Queses all terrain entities for rendering via the terrain pipeline.
#[allow(clippy::too_many_arguments)]
pub(crate) fn queue_terrain<M: Material>(
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    msaa: Res<Msaa>,
    debug: Option<Res<DebugTerrain>>,
    render_materials: Res<RenderMaterials<M>>,
    terrain_pipeline: Res<TerrainRenderPipeline<M>>,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TerrainRenderPipeline<M>>>,
    mut view_query: Query<&mut RenderPhase<Opaque3d>>,
    terrain_query: Query<(Entity, &Handle<M>), With<Terrain>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let draw_function = draw_functions.read().get_id::<DrawTerrain<M>>().unwrap();

    for mut opaque_phase in view_query.iter_mut() {
        for (entity, material) in terrain_query.iter() {
            if let Some(material) = render_materials.get(material) {
                let mut flags = TerrainPipelineFlags::from_msaa_samples(msaa.samples());

                if let Some(debug) = &debug {
                    flags |= TerrainPipelineFlags::from_debug(debug);
                } else {
                    flags |= TerrainPipelineFlags::LIGHTING
                        | TerrainPipelineFlags::SHOW_NODES
                        | TerrainPipelineFlags::MESH_MORPH
                        | TerrainPipelineFlags::SAMPLE_GRAD;
                }

                let key = TerrainPipelineKey {
                    flags,
                    bind_group_data: material.key.clone(),
                };

                let pipeline_id = pipelines.specialize(&pipeline_cache, &terrain_pipeline, key);

                opaque_phase.add(Opaque3d {
                    entity,
                    pipeline: pipeline_id,
                    draw_function,
                    distance: f32::MIN, // draw terrain first
                });
            }
        }
    }
}

/// This plugin adds a custom material for a terrain.
///
/// It can be used to render the terrain using a custom vertex and fragment shader.
pub struct TerrainMaterialPlugin<M: Material>(PhantomData<M>);

impl<M: Material> Default for TerrainMaterialPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: Material> Plugin for TerrainMaterialPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
       
         // Todo: don't use MaterialPlugin, but do the configuration here
         app.add_plugins(MaterialPlugin::<M>::default());
    }

    //this may not be correct !! 
    fn finish(&self, app: &mut App) {
        


        let mut render_app = app.get_sub_app_mut(RenderApp).unwrap();
       // if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                // .init_resource::<ExtractedMaterials<M>>()
                // .init_resource::<RenderMaterials<M>>()
                // .add_system_to_stage(RenderStage::Extract, extract_materials::<M>)
                // .add_system_to_stage(
                //     RenderStage::Prepare,
                //     prepare_materials::<M>.after(PrepareAssetLabel::PreAssetPrepare),
                // )
                .add_render_command::<Opaque3d, DrawTerrain<M>>()
                .init_resource::<TerrainRenderPipeline<M>>()
                .init_resource::<SpecializedRenderPipelines<TerrainRenderPipeline<M>>>()
                .add_systems(bevy::render::Render,queue_terrain::<M>.in_set(RenderSet::Queue));
       // }
    }

}
