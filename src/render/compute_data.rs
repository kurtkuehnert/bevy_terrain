use crate::render::layouts::PATCH_LIST_LAYOUT;
use crate::{
    render::resources::TerrainResources, GpuQuadtree, Terrain, TerrainComputePipelines,
    TerrainConfig, TerrainView, TerrainViewComponents,
};
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
};

pub struct TerrainComputeData {
    pub(crate) refinement_count: usize,
    pub(crate) indirect_buffer: Buffer,
    pub(crate) prepare_indirect_bind_group: BindGroup,
    pub(crate) tessellation_bind_group: BindGroup,
    pub(crate) patch_list_bind_group: BindGroup,
}

impl TerrainComputeData {
    fn new(
        device: &RenderDevice,
        config: &TerrainConfig,
        resources: &TerrainResources,
        gpu_quadtree: &GpuQuadtree,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Self {
        let prepare_indirect_bind_group = Self::create_prepare_indirect_bind_group(
            device,
            resources,
            &compute_pipelines.prepare_indirect_layout,
        );
        let tessellation_bind_group = Self::create_tesselation_bind_group(
            device,
            resources,
            &compute_pipelines.tessellation_layout,
        );
        let patch_list_bind_group = Self::create_patch_list_bind_group(
            device,
            resources,
            gpu_quadtree,
            &device.create_bind_group_layout(&PATCH_LIST_LAYOUT),
        );

        Self {
            refinement_count: config.refinement_count as usize,
            indirect_buffer: resources.indirect_buffer.clone(),
            prepare_indirect_bind_group,
            tessellation_bind_group,
            patch_list_bind_group,
        }
    }

    fn create_tesselation_bind_group(
        device: &RenderDevice,
        resources: &TerrainResources,
        layout: &BindGroupLayout,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: resources.config_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: resources.parameter_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: resources.temporary_patch_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: resources.final_patch_buffer.as_entire_binding(),
                },
            ],
            layout,
        })
    }

    fn create_prepare_indirect_bind_group(
        device: &RenderDevice,
        resources: &TerrainResources,
        layout: &BindGroupLayout,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: resources.indirect_buffer.as_entire_binding(),
            }],
            layout,
        })
    }

    fn create_patch_list_bind_group(
        device: &RenderDevice,
        resources: &TerrainResources,
        gpu_quadtree: &GpuQuadtree,
        layout: &BindGroupLayout,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: resources.final_patch_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&gpu_quadtree.view),
                },
            ],
            layout,
        })
    }
}

/// Runs in queue.
pub(crate) fn initialize_terrain_compute_data(
    device: Res<RenderDevice>,
    compute_pipelines: Res<TerrainComputePipelines>,
    terrain_resources: ResMut<TerrainViewComponents<TerrainResources>>,
    gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
    mut terrain_compute_data: ResMut<TerrainViewComponents<TerrainComputeData>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<(Entity, &TerrainConfig), With<Terrain>>,
) {
    for (terrain, config) in terrain_query.iter() {
        for view in view_query.iter() {
            let resources = terrain_resources.get(&(terrain, view)).unwrap();
            let gpu_quadtree = gpu_quadtrees.get(&(terrain, view)).unwrap();
            terrain_compute_data.insert(
                (terrain, view),
                TerrainComputeData::new(
                    &device,
                    &config,
                    &resources,
                    &gpu_quadtree,
                    &compute_pipelines,
                ),
            );
        }
    }
}
