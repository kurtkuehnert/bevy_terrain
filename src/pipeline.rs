use crate::material::{SetTerrainMaterialBindGroup, TerrainMaterial};
use bevy::{
    core_pipeline::Opaque3d,
    ecs::{
        query::QueryItem,
        system::lifetimeless::{Read, SQuery},
        system::{lifetimeless::SRes, SystemParamItem},
    },
    pbr::{MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        mesh::GpuBufferInfo,
        render_asset::RenderAssets,
        render_component::ExtractComponent,
        render_phase::{
            DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::{
            internal::bytemuck::{Pod, Zeroable},
            *,
        },
        renderer::RenderDevice,
    },
};

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct TileData {
    pub(crate) position: UVec2,
    pub(crate) size: u32,
    pub(crate) range: f32,
    pub(crate) color: Vec4,
}

#[derive(Component)]
pub struct GpuTerrainData {
    buffer: Buffer,
    length: usize,
}

#[derive(Clone, Default, Component)]
pub struct TerrainData {
    pub(crate) data: Vec<TileData>,
}

impl TerrainData {
    fn vertex_buffer_layout() -> VertexBufferLayout {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<TileData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Uint32x2,
                    offset: 0,
                    shader_location: 3,
                },
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: VertexFormat::Uint32x2.size(),
                    shader_location: 4,
                },
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: VertexFormat::Uint32x2.size() + VertexFormat::Uint32.size(),
                    shader_location: 5,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Uint32x2.size()
                        + VertexFormat::Uint32.size()
                        + VertexFormat::Float32.size(),
                    shader_location: 6,
                },
            ],
        }
    }
}

impl ExtractComponent for TerrainData {
    type Query = &'static TerrainData;
    type Filter = ();

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

pub(crate) fn prepare_terrain(
    mut commands: Commands,
    terrain_query: Query<(Entity, &TerrainData)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, terrain_data) in terrain_query.iter() {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("terrain data buffer"),
            contents: bytemuck::cast_slice(terrain_data.data.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        commands.entity(entity).insert(GpuTerrainData {
            buffer,
            length: terrain_data.data.len(),
        });
    }
}

pub(crate) fn queue_terrain(
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    terrain_pipeline: Res<TerrainPipeline>,
    msaa: Res<Msaa>,
    meshes: Res<RenderAssets<Mesh>>,
    mut pipelines: ResMut<SpecializedPipelines<TerrainPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut view_query: Query<&mut RenderPhase<Opaque3d>>,
    terrain_query: Query<(Entity, &Handle<Mesh>), With<TerrainData>>,
) {
    let draw_function = draw_functions.read().get_id::<DrawTerrain>().unwrap();

    for mut opaque_phase in view_query.iter_mut() {
        for (entity, mesh) in terrain_query.iter() {
            let topology = meshes.get(mesh).unwrap().primitive_topology;

            let key = MeshPipelineKey::from_msaa_samples(msaa.samples)
                | MeshPipelineKey::from_primitive_topology(topology);
            let pipeline = pipelines.specialize(&mut pipeline_cache, &terrain_pipeline, key);

            opaque_phase.add(Opaque3d {
                entity,
                pipeline,
                draw_function,
                distance: f32::MIN,
            });
        }
    }
}

pub struct TerrainPipeline {
    pub(crate) mesh_pipeline: MeshPipeline,
    pub(crate) material_layout: BindGroupLayout,
    pub(crate) shader: Handle<Shader>,
}

impl FromWorld for TerrainPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let shader = asset_server.load("shaders/terrain.wgsl");
        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let material_layout = TerrainMaterial::bind_group_layout(render_device);

        TerrainPipeline {
            mesh_pipeline: mesh_pipeline.clone(),
            material_layout: material_layout.clone(),
            shader,
        }
    }
}

impl SpecializedPipeline for TerrainPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self.mesh_pipeline.specialize(key);
        descriptor.vertex.shader = self.shader.clone();
        descriptor
            .vertex
            .buffers
            .push(TerrainData::vertex_buffer_layout());
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.material_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
        ]);

        descriptor
    }
}

pub(crate) type DrawTerrain = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetTerrainMaterialBindGroup<1>,
    SetMeshBindGroup<2>,
    DrawTerrainCommand,
);

pub(crate) struct DrawTerrainCommand;

impl EntityRenderCommand for DrawTerrainCommand {
    type Param = (
        SRes<RenderAssets<Mesh>>,
        SQuery<(Read<GpuTerrainData>, Read<Handle<Mesh>>)>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (terrain_buffer, mesh) = terrain_query.get(item).unwrap();

        let gpu_mesh = match meshes.into_inner().get(mesh) {
            Some(gpu_mesh) => gpu_mesh,
            None => return RenderCommandResult::Failure,
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, terrain_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, 0..terrain_buffer.length as u32);
            }
            GpuBufferInfo::NonIndexed { vertex_count } => {
                pass.draw_indexed(0..*vertex_count, 0, 0..terrain_buffer.length as u32);
            }
        }

        RenderCommandResult::Success
    }
}
