use crate::shaders::DEFAULT_SHADER;
use crate::{
    debug::DebugTerrain,
    render::{
        terrain_bind_group::{create_terrain_layout, SetTerrainBindGroup},
        terrain_view_bind_group::{
            create_terrain_view_layout, DrawTerrainCommand, SetTerrainViewBindGroup,
        },
    },
    terrain::{Terrain, TerrainConfig},
    terrain_view::TerrainView,
};
use bevy::{
    core_pipeline::core_3d::{Opaque3d, Opaque3dBinKey},
    pbr::{
        MaterialPipeline, MeshPipeline, MeshPipelineViewLayoutKey, PreparedMaterial,
        RenderMaterialInstances, SetMaterialBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::render_phase::{BinnedRenderPhaseType, ViewBinnedRenderPhases},
    render::{
        extract_instances::ExtractInstancesPlugin,
        render_asset::{prepare_assets, RenderAssetPlugin, RenderAssets},
        render_phase::{AddRenderCommand, DrawFunctions, SetItemPipeline},
        render_resource::*,
        renderer::RenderDevice,
        texture::{BevyDefault, GpuImage},
        Render, RenderApp, RenderSet,
    },
};
use std::{hash::Hash, marker::PhantomData};

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
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct TerrainPipelineFlags: u32 {
        const NONE               = 0;
        const SPHERICAL          = (1 <<  0);
        const WIREFRAME          = (1 <<  1);
        const SHOW_LOD           = (1 <<  2);
        const SHOW_UV            = (1 <<  3);
        const SHOW_TILES         = (1 <<  4);
        const SHOW_QUADTREE      = (1 <<  5);
        const SHOW_PIXELS        = (1 <<  6);
        const SHOW_NORMALS       = (1 <<  7);
        const MORPH              = (1 <<  8);
        const BLEND              = (1 <<  9);
        const QUADTREE_LOD       = (1 << 10);
        const ALBEDO             = (1 << 11);
        const BRIGHT             = (1 << 12);
        const LIGHTING           = (1 << 13);
        const SAMPLE_GRAD        = (1 << 14);
        const TEST1              = (1 << 15);
        const TEST2              = (1 << 16);
        const TEST3              = (1 << 17);

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
        if debug.show_lod {
            key |= TerrainPipelineFlags::SHOW_LOD;
        }
        if debug.show_uv {
            key |= TerrainPipelineFlags::SHOW_UV;
        }
        if debug.show_tiles {
            key |= TerrainPipelineFlags::SHOW_TILES;
        }
        if debug.show_quadtree {
            key |= TerrainPipelineFlags::SHOW_QUADTREE;
        }
        if debug.show_pixels {
            key |= TerrainPipelineFlags::SHOW_PIXELS;
        }
        if debug.show_normals {
            key |= TerrainPipelineFlags::SHOW_NORMALS;
        }
        if debug.morph {
            key |= TerrainPipelineFlags::MORPH;
        }
        if debug.blend {
            key |= TerrainPipelineFlags::BLEND;
        }
        if debug.quadtree_lod {
            key |= TerrainPipelineFlags::QUADTREE_LOD;
        }
        if debug.albedo {
            key |= TerrainPipelineFlags::ALBEDO;
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
        ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }

    pub fn polygon_mode(&self) -> PolygonMode {
        match (self.bits() & TerrainPipelineFlags::WIREFRAME.bits()) != 0 {
            true => PolygonMode::Line,
            false => PolygonMode::Fill,
        }
    }

    pub fn shader_defs(&self) -> Vec<ShaderDefVal> {
        let mut shader_defs = Vec::new();

        if (self.bits() & TerrainPipelineFlags::SPHERICAL.bits()) != 0 {
            shader_defs.push("SPHERICAL".into());
        }
        if (self.bits() & TerrainPipelineFlags::SHOW_LOD.bits()) != 0 {
            shader_defs.push("SHOW_LOD".into());
        }
        if (self.bits() & TerrainPipelineFlags::SHOW_UV.bits()) != 0 {
            shader_defs.push("SHOW_UV".into());
        }
        if (self.bits() & TerrainPipelineFlags::SHOW_TILES.bits()) != 0 {
            shader_defs.push("SHOW_TILES".into());
        }
        if (self.bits() & TerrainPipelineFlags::SHOW_QUADTREE.bits()) != 0 {
            shader_defs.push("SHOW_QUADTREE".into());
        }
        if (self.bits() & TerrainPipelineFlags::SHOW_PIXELS.bits()) != 0 {
            shader_defs.push("SHOW_PIXELS".into())
        }
        if (self.bits() & TerrainPipelineFlags::SHOW_NORMALS.bits()) != 0 {
            shader_defs.push("SHOW_NORMALS".into())
        }
        if (self.bits() & TerrainPipelineFlags::MORPH.bits()) != 0 {
            shader_defs.push("MORPH".into());
        }
        if (self.bits() & TerrainPipelineFlags::BLEND.bits()) != 0 {
            shader_defs.push("BLEND".into());
        }
        if (self.bits() & TerrainPipelineFlags::QUADTREE_LOD.bits()) != 0 {
            shader_defs.push("QUADTREE_LOD".into());
        }
        if (self.bits() & TerrainPipelineFlags::ALBEDO.bits()) != 0 {
            shader_defs.push("ALBEDO".into());
        }
        if (self.bits() & TerrainPipelineFlags::BRIGHT.bits()) != 0 {
            shader_defs.push("BRIGHT".into());
        }
        if (self.bits() & TerrainPipelineFlags::LIGHTING.bits()) != 0 {
            shader_defs.push("LIGHTING".into());
        }
        if (self.bits() & TerrainPipelineFlags::SAMPLE_GRAD.bits()) != 0 {
            shader_defs.push("SAMPLE_GRAD".into());
        }
        if (self.bits() & TerrainPipelineFlags::TEST1.bits()) != 0 {
            shader_defs.push("TEST1".into());
        }
        if (self.bits() & TerrainPipelineFlags::TEST2.bits()) != 0 {
            shader_defs.push("TEST2".into());
        }
        if (self.bits() & TerrainPipelineFlags::TEST3.bits()) != 0 {
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

        let view_layout = mesh_pipeline
            .get_view_layout(MeshPipelineViewLayoutKey::empty())
            .clone();
        let view_layout_multisampled = mesh_pipeline
            .get_view_layout(MeshPipelineViewLayoutKey::MULTISAMPLED)
            .clone();
        let terrain_layout = create_terrain_layout(device);
        let terrain_view_layout = create_terrain_view_layout(device);
        let material_layout = M::bind_group_layout(device);

        let vertex_shader = match M::vertex_shader() {
            ShaderRef::Default => asset_server.load(DEFAULT_SHADER),
            ShaderRef::Handle(handle) => handle,
            ShaderRef::Path(path) => asset_server.load(path),
        };

        let fragment_shader = match M::fragment_shader() {
            ShaderRef::Default => asset_server.load(DEFAULT_SHADER),
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

        let mut bind_group_layout = match key.flags.msaa_samples() {
            1 => vec![self.view_layout.clone()],
            _ => {
                shader_defs.push("MULTISAMPLED".into());
                vec![self.view_layout_multisampled.clone()]
            }
        };

        bind_group_layout.push(self.terrain_layout.clone());
        bind_group_layout.push(self.terrain_view_layout.clone());
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
    SetTerrainBindGroup<1>,
    SetTerrainViewBindGroup<2>,
    SetMaterialBindGroup<M, 3>,
    DrawTerrainCommand,
);

/// Queses all terrain entities for rendering via the terrain pipeline.
#[allow(clippy::too_many_arguments)]
pub(crate) fn queue_terrain<M: Material>(
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    msaa: Res<Msaa>,
    debug: Option<Res<DebugTerrain>>,
    render_materials: Res<RenderAssets<PreparedMaterial<M>>>,
    pipeline_cache: Res<PipelineCache>,
    terrain_pipeline: Res<TerrainRenderPipeline<M>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TerrainRenderPipeline<M>>>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<&TerrainConfig, With<Terrain>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for view in view_query.iter() {
        let phase = opaque_render_phases.get_mut(&view).unwrap();
        let draw_function = draw_functions.read().get_id::<DrawTerrain<M>>().unwrap();

        for (&terrain, &material_id) in render_material_instances.iter() {
            let config = terrain_query.get(terrain).unwrap();
            if let Some(material) = render_materials.get(material_id) {
                let mut flags = TerrainPipelineFlags::from_msaa_samples(msaa.samples());

                if config.model.spherical {
                    flags |= TerrainPipelineFlags::SPHERICAL;
                }

                if let Some(debug) = &debug {
                    flags |= TerrainPipelineFlags::from_debug(debug);
                } else {
                    flags |= TerrainPipelineFlags::LIGHTING
                        | TerrainPipelineFlags::MORPH
                        | TerrainPipelineFlags::BLEND
                        | TerrainPipelineFlags::SAMPLE_GRAD;
                }

                let key = TerrainPipelineKey {
                    flags,
                    bind_group_data: material.key.clone(),
                };

                let pipeline = pipelines.specialize(&pipeline_cache, &terrain_pipeline, key);

                phase.add(
                    Opaque3dBinKey {
                        pipeline,
                        draw_function,
                        asset_id: material_id.untyped(),
                        material_bind_group_id: None,
                        lightmap_image: None,
                    },
                    terrain,
                    BinnedRenderPhaseType::NonMesh,
                );
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
        app.init_asset::<M>().add_plugins((
            ExtractInstancesPlugin::<AssetId<M>>::extract_visible(),
            RenderAssetPlugin::<PreparedMaterial<M>, GpuImage>::default(),
        ));

        app.sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain<M>>()
            .add_systems(
                Render,
                queue_terrain::<M>
                    .in_set(RenderSet::QueueMeshes)
                    .after(prepare_assets::<PreparedMaterial<M>>),
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<TerrainRenderPipeline<M>>()
            .init_resource::<SpecializedRenderPipelines<TerrainRenderPipeline<M>>>()
            .init_resource::<MaterialPipeline<M>>(); // prepare assets depends on this to access the material layout
    }
}
