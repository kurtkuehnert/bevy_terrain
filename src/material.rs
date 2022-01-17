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
        render_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            internal::bytemuck::{Pod, Zeroable},
            *,
        },
        renderer::RenderDevice,
        RenderApp, RenderStage,
    },
};

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct TileData {
    pub position: UVec2,
    pub size: u32,
    pub range: f32,
    pub color: Vec4,
}

#[derive(Clone, Default, Component)]
pub struct InstanceData {
    pub instance_data: Vec<TileData>,
    pub wireframe: bool,
    pub sparse: bool,
}

impl ExtractComponent for InstanceData {
    type Query = &'static InstanceData;
    type Filter = ();

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<InstanceData>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawTerrain>()
            .init_resource::<TerrainPipeline>()
            .init_resource::<SpecializedPipelines<TerrainPipeline>>()
            .add_system_to_stage(RenderStage::Prepare, prepare_terrain)
            .add_system_to_stage(RenderStage::Queue, queue_terrain);
    }
}

fn queue_terrain(
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    terrain_pipeline: Res<TerrainPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedPipelines<TerrainPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mesh_query: Query<(Entity, &InstanceData), With<Handle<Mesh>>>,
    mut view_query: Query<&mut RenderPhase<Opaque3d>>,
) {
    let draw_function = draw_functions.read().get_id::<DrawTerrain>().unwrap();

    // Todo: query wireframe info from mesh
    let key = MeshPipelineKey::from_msaa_samples(msaa.samples)
        | MeshPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);
    let pipeline = pipelines.specialize(&mut pipeline_cache, &terrain_pipeline, key);
    let pipeline_wireframe = pipelines.specialize(
        &mut pipeline_cache,
        &terrain_pipeline,
        MeshPipelineKey::from_msaa_samples(msaa.samples)
            | MeshPipelineKey::from_primitive_topology(PrimitiveTopology::LineList),
    );

    for mut opaque_phase in view_query.iter_mut() {
        for (entity, terrain_data) in mesh_query.iter() {
            let pipeline = if terrain_data.wireframe {
                pipeline_wireframe
            } else {
                pipeline
            };

            opaque_phase.add(Opaque3d {
                entity,
                pipeline,
                draw_function,
                distance: f32::MIN,
            });
        }
    }
}

#[derive(Component)]
pub struct GpuTerrainBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_terrain(
    mut commands: Commands,
    terrain_query: Query<(Entity, &InstanceData)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, terrain_data) in terrain_query.iter() {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("terrain data buffer"),
            contents: bytemuck::cast_slice(terrain_data.instance_data.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        commands.entity(entity).insert(GpuTerrainBuffer {
            buffer,
            length: terrain_data.instance_data.len(),
        });
    }
}

pub struct TerrainPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
}

impl FromWorld for TerrainPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let shader = asset_server.load("shaders/terrain.wgsl");
        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();

        TerrainPipeline {
            shader,
            mesh_pipeline: mesh_pipeline.clone(),
        }
    }
}

impl SpecializedPipeline for TerrainPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self.mesh_pipeline.specialize(key);
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
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
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
        ]);

        descriptor
    }
}

type DrawTerrain = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawTerrainCommand,
);

pub struct DrawTerrainCommand;

impl EntityRenderCommand for DrawTerrainCommand {
    type Param = (
        SRes<RenderAssets<Mesh>>,
        SQuery<Read<Handle<Mesh>>>,
        SQuery<Read<GpuTerrainBuffer>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, mesh_query, terrain_buffer_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh = mesh_query.get(item).unwrap();
        let terrain_buffer = terrain_buffer_query.get(item).unwrap();

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
