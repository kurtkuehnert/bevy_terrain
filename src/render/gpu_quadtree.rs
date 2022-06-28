use crate::{
    quadtree::{NodeUpdate, Quadtree},
    render::layouts::NODE_UPDATE_SIZE,
    terrain::Terrain,
    terrain_view::TerrainView,
    TerrainComputePipelines, TerrainViewComponents,
};
use bevy::{
    core::cast_slice,
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        RenderWorld,
    },
};
use std::mem;

/// Stores the GPU representation of the [`Quadtree`] alongside the data to update it.
#[derive(Component)]
pub struct GpuQuadtree {
    pub(crate) quadtree_view: TextureView,
    pub(crate) update_bind_group: BindGroup,
    pub(crate) update_buffer: Buffer,
    pub(crate) node_updates: Vec<NodeUpdate>,
}

impl GpuQuadtree {
    fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        quadtree: &Quadtree,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Self {
        let size = quadtree.lod_count * quadtree.node_count * quadtree.node_count;
        let data = vec![u64::MAX; size as usize];

        let quadtree_texture = device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: "quadtree_texture".into(),
                size: Extent3d {
                    width: quadtree.node_count,
                    height: quadtree.node_count,
                    depth_or_array_layers: quadtree.lod_count,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Uint,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            },
            cast_slice(&data),
        );

        let quadtree_view = quadtree_texture.create_view(&TextureViewDescriptor {
            label: "quadtree_view".into(),
            ..default()
        });

        let update_buffer = device.create_buffer(&BufferDescriptor {
            label: "node_updates_buffer".into(),
            size: NODE_UPDATE_SIZE * 10 * size as BufferAddress, // Todo: calculate correctly
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let update_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "update_quadtree_bind_group".into(),
            layout: &compute_pipelines.update_quadtree_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&quadtree_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: update_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            quadtree_view,
            update_bind_group,
            update_buffer,
            node_updates: default(),
        }
    }
}

/// Initializes the [`GpuQuadtree`] of newly created terrains.
pub(crate) fn initialize_gpu_quadtree(
    mut render_world: ResMut<RenderWorld>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<Entity, Added<Terrain>>,
) {
    let mut gpu_quadtrees = render_world
        .remove_resource::<TerrainViewComponents<GpuQuadtree>>()
        .unwrap();
    let compute_pipelines = render_world.resource::<TerrainComputePipelines>();

    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            let quadtree = quadtrees.get(&(terrain, view)).unwrap();

            gpu_quadtrees.insert(
                (terrain, view),
                GpuQuadtree::new(&device, &queue, &quadtree, &compute_pipelines),
            );
        }
    }

    render_world.insert_resource(gpu_quadtrees);
}

/// Extracts the new nodes updates for all [`GpuQuadtree`]s by copying them over from their
/// corresponding [`Quadtree`]s.
pub(crate) fn update_gpu_quadtree(
    mut render_world: ResMut<RenderWorld>,
    mut quadtrees: ResMut<TerrainViewComponents<Quadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    let mut gpu_quadtrees = render_world.resource_mut::<TerrainViewComponents<GpuQuadtree>>();

    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            if let Some((quadtree, gpu_quadtree)) = quadtrees
                .get_mut(&(terrain, view))
                .zip(gpu_quadtrees.get_mut(&(terrain, view)))
            {
                gpu_quadtree.node_updates.clear();
                mem::swap(&mut quadtree.node_updates, &mut gpu_quadtree.node_updates);
            }
        }
    }
}

/// Queues the [`NodeUpdate`]s generated this frame for the quadtree update pipeline,
/// by filling the node update buffers with them.
pub(crate) fn queue_quadtree_updates(
    queue: Res<RenderQueue>,
    mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            if let Some(gpu_quadtree) = gpu_quadtrees.get_mut(&(terrain, view)) {
                queue.write_buffer(
                    &gpu_quadtree.update_buffer,
                    0,
                    cast_slice(&gpu_quadtree.node_updates),
                );
            }
        }
    }
}
