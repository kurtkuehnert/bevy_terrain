use crate::{
    render::terrain_data::{TerrainData, PATCH_SIZE},
    terrain::TerrainConfigUniform,
};
use bevy::{
    core_pipeline::Opaque3d,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        mesh::GpuBufferInfo,
        render_asset::RenderAssets,
        render_phase::{
            DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::*,
        renderer::RenderDevice,
    },
};

pub(crate) const TERRAIN_DATA_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        // terrain config
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(TerrainConfigUniform::buffer_size()),
            },
            count: None,
        },
        // height atlas
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Uint,
                view_dimension: TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        // Todo: add Chunk Map here
    ],
};

pub(crate) const PATCH_LIST_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::VERTEX,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(PATCH_SIZE),
        },
        count: None,
    }],
};

/// The pipeline used to render the terrain entities.
pub struct TerrainPipeline {
    pub(crate) mesh_pipeline: MeshPipeline,
    pub(crate) terrain_data_layout: BindGroupLayout,
    pub(crate) patch_list_layout: BindGroupLayout,
    pub(crate) shader: Handle<Shader>, // Todo: make fragment shader customizable
}

impl FromWorld for TerrainPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.get_resource::<RenderDevice>().unwrap();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap().clone();
        let terrain_data_layout = device.create_bind_group_layout(&TERRAIN_DATA_LAYOUT);
        let patch_list_layout = device.create_bind_group_layout(&PATCH_LIST_LAYOUT);
        let shader = asset_server.load("shaders/terrain.wgsl");

        TerrainPipeline {
            mesh_pipeline,
            terrain_data_layout,
            patch_list_layout,
            shader,
        }
    }
}

impl SpecializedPipeline for TerrainPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self.mesh_pipeline.specialize(key);
        descriptor.vertex.shader = self.shader.clone();
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
            self.terrain_data_layout.clone(),
            self.patch_list_layout.clone(),
        ]);

        descriptor
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

pub(crate) struct DrawTerrainCommand;

impl EntityRenderCommand for DrawTerrainCommand {
    type Param = (
        SRes<RenderAssets<Mesh>>,
        SRes<RenderAssets<TerrainData>>,
        SQuery<(Read<Handle<Mesh>>, Read<Handle<TerrainData>>)>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, terrain_data, terrain_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (mesh, handle) = terrain_query.get(item).unwrap();
        let gpu_mesh = meshes.into_inner().get(mesh).unwrap();
        let gpu_terrain_data = terrain_data.into_inner().get(handle).unwrap();

        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                ..
            } => {
                pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.inner()
                    .draw_indexed_indirect(&gpu_terrain_data.indirect_buffer, 0);

                RenderCommandResult::Success
            }
            _ => RenderCommandResult::Failure,
        }
    }
}

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

/// Queses all terrain entities for rendering via the terrain pipeline.
pub(crate) fn queue_terrain(
    terrain_pipeline: Res<TerrainPipeline>,
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    msaa: Res<Msaa>,
    meshes: Res<RenderAssets<Mesh>>,
    mut pipelines: ResMut<SpecializedPipelines<TerrainPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut view_query: Query<&mut RenderPhase<Opaque3d>>,
    terrain_query: Query<(Entity, &Handle<Mesh>), With<Handle<TerrainData>>>,
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
                distance: f32::MIN, // draw terrain first
            });
        }
    }
}
