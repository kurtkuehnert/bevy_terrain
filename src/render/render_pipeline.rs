use crate::render::layouts::PATCH_LIST_LAYOUT;
use bevy::{
    pbr::MeshPipeline,
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice, texture::BevyDefault},
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
pub struct TerrainRenderPipeline {
    pub(crate) view_layout: BindGroupLayout,
    pub(crate) mesh_layout: BindGroupLayout,
    pub(crate) terrain_data_layouts: Vec<BindGroupLayout>,
    pub(crate) patch_list_layout: BindGroupLayout,
    pub(crate) shader: Handle<Shader>, // Todo: make fragment shader customizable
}

impl FromWorld for TerrainRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();
        let mesh_pipeline = world.resource::<MeshPipeline>();

        let view_layout = mesh_pipeline.view_layout.clone();
        let mesh_layout = mesh_pipeline.mesh_layout.clone();
        let patch_list_layout = device.create_bind_group_layout(&PATCH_LIST_LAYOUT);
        let shader = asset_server.load("shaders/terrain.wgsl");

        Self {
            view_layout,
            mesh_layout,
            terrain_data_layouts: Vec::new(),
            patch_list_layout,
            shader,
        }
    }
}

impl SpecializedRenderPipeline for TerrainRenderPipeline {
    type Key = TerrainPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: None,
            layout: Some(vec![
                self.view_layout.clone(),
                self.mesh_layout.clone(),
                self.terrain_data_layouts[0].clone(), // Todo: do this properly for multiple maps
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
