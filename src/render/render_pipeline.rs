use crate::{render::layouts::TERRAIN_VIEW_LAYOUT, DebugTerrain};
use bevy::{
    pbr::MeshPipeline,
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice, texture::BevyDefault},
};

pub struct TerrainPipelineConfig {
    shader: String,
}

impl Default for TerrainPipelineConfig {
    fn default() -> Self {
        Self {
            shader: "shaders/terrain.wgsl".into(),
        }
    }
}

bitflags::bitflags! {
#[repr(transparent)]
pub struct TerrainPipelineKey: u32 {
    const NONE               = 0;
    const WIREFRAME          = (1 << 0);
    const SHOW_PATCHES       = (1 << 1);
    const SHOW_LOD           = (1 << 2);
    const SHOW_UV            = (1 << 3);
    const CIRCULAR_LOD       = (1 << 4);
    const MESH_MORPH         = (1 << 5);
    const ALBEDO             = (1 << 6);
    const BRIGHT             = (1 << 7);
    const LIGHTING           = (1 << 8);
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

    pub fn from_debug(debug: &DebugTerrain) -> Self {
        let mut key = TerrainPipelineKey::NONE;

        if debug.wireframe {
            key |= TerrainPipelineKey::WIREFRAME;
        }

        if debug.show_patches {
            key |= TerrainPipelineKey::SHOW_PATCHES;
        }
        if debug.show_lod {
            key |= TerrainPipelineKey::SHOW_LOD;
        }
        if debug.show_uv {
            key |= TerrainPipelineKey::SHOW_UV;
        }

        if debug.circular_lod {
            key |= TerrainPipelineKey::CIRCULAR_LOD;
        }
        if debug.mesh_morph {
            key |= TerrainPipelineKey::MESH_MORPH;
        }

        if debug.albedo {
            key |= TerrainPipelineKey::ALBEDO;
        }
        if debug.bright {
            key |= TerrainPipelineKey::BRIGHT;
        }
        if debug.lighting {
            key |= TerrainPipelineKey::LIGHTING;
        }

        key
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }

    pub fn polygon_mode(&self) -> PolygonMode {
        match (self.bits & TerrainPipelineKey::WIREFRAME.bits) != 0 {
            true => PolygonMode::Line,
            false => PolygonMode::Fill,
        }
    }

    pub fn shader_defs(&self) -> Vec<String> {
        let mut shader_defs = Vec::new();

        if (self.bits & TerrainPipelineKey::SHOW_PATCHES.bits) != 0 {
            shader_defs.push("SHOW_PATCHES".to_string());
        }
        if (self.bits & TerrainPipelineKey::SHOW_LOD.bits) != 0 {
            shader_defs.push("SHOW_LOD".to_string());
        }
        if (self.bits & TerrainPipelineKey::SHOW_UV.bits) != 0 {
            shader_defs.push("SHOW_UV".to_string());
        }

        if (self.bits & TerrainPipelineKey::CIRCULAR_LOD.bits) != 0 {
            shader_defs.push("CIRCULAR_LOD".to_string());
        }
        if (self.bits & TerrainPipelineKey::MESH_MORPH.bits) != 0 {
            shader_defs.push("MESH_MORPH".to_string());
        }

        if (self.bits & TerrainPipelineKey::ALBEDO.bits) != 0 {
            shader_defs.push("ALBEDO".to_string());
        }
        if (self.bits & TerrainPipelineKey::BRIGHT.bits) != 0 {
            shader_defs.push("BRIGHT".to_string());
        }
        if (self.bits & TerrainPipelineKey::LIGHTING.bits) != 0 {
            shader_defs.push("LIGHTING".to_string());
        }

        shader_defs
    }
}

/// The pipeline used to render the terrain entities.
pub struct TerrainRenderPipeline {
    pub(crate) view_layout: BindGroupLayout,
    pub(crate) mesh_layout: BindGroupLayout,
    pub(crate) terrain_layouts: Vec<BindGroupLayout>,
    pub(crate) terrain_view_layout: BindGroupLayout,
    pub(crate) shader: Handle<Shader>,
}

impl FromWorld for TerrainRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();
        let mesh_pipeline = world.resource::<MeshPipeline>();

        let view_layout = mesh_pipeline.view_layout.clone();
        let mesh_layout = mesh_pipeline.mesh_layout.clone();
        let terrain_view_layout = device.create_bind_group_layout(&TERRAIN_VIEW_LAYOUT);
        let shader = asset_server.load(&world.resource::<TerrainPipelineConfig>().shader);

        Self {
            view_layout,
            mesh_layout,
            terrain_layouts: Vec::new(),
            terrain_view_layout,
            shader,
        }
    }
}

impl SpecializedRenderPipeline for TerrainRenderPipeline {
    type Key = TerrainPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = key.shader_defs();

        RenderPipelineDescriptor {
            label: None,
            layout: Some(vec![
                self.view_layout.clone(),
                self.terrain_view_layout.clone(),
                self.terrain_layouts[0].clone(), // Todo: do this properly for multiple maps
                self.mesh_layout.clone(),
            ]),
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: Vec::new(),
            },
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: key.polygon_mode(),
                conservative: false,
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
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
