use crate::{
    config::TerrainConfig,
    quadtree::{NodeUpdate, Quadtree},
    render::{layouts::NODE_UPDATE_SIZE, PersistentComponents},
    Terrain, TerrainComputePipelines,
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
    pub(crate) view: TextureView,
    pub(crate) update_bind_group: BindGroup,
    pub(crate) update_buffer: Buffer,
    pub(crate) node_updates: Vec<NodeUpdate>,
}

impl GpuQuadtree {
    fn new(
        config: &TerrainConfig,
        device: &RenderDevice,
        queue: &RenderQueue,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Self {
        let (view, update_bind_group, update_buffer) =
            Self::create_quadtree(config, device, queue, compute_pipelines);

        Self {
            view,
            update_bind_group,
            update_buffer,
            node_updates: default(),
        }
    }

    fn create_quadtree(
        config: &TerrainConfig,
        device: &RenderDevice,
        queue: &RenderQueue,
        compute_pipelines: &TerrainComputePipelines,
    ) -> (TextureView, BindGroup, Buffer) {
        let size = config.lod_count * config.node_count * config.node_count;
        let data = vec![u64::MAX; size as usize];

        let quadtree = device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: "quadtree_texture".into(),
                size: Extent3d {
                    width: config.node_count,
                    height: config.node_count,
                    depth_or_array_layers: config.lod_count,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Uint,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            },
            cast_slice(&data),
        );

        let view = quadtree.create_view(&TextureViewDescriptor {
            label: "quadtree_view".into(),
            ..default()
        });

        let update_buffer = device.create_buffer(&BufferDescriptor {
            label: "quadtree_activation_buffer".into(),
            size: NODE_UPDATE_SIZE * 10 * size as BufferAddress, // Todo: calculate correctly
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let update_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "quadtree_update_bind_group".into(),
            layout: &compute_pipelines.update_quadtree_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: update_buffer.as_entire_binding(),
                },
            ],
        });

        (view, update_bind_group, update_buffer)
    }
}

/// Initializes the [`GpuQuadtree`] of newly created terrains.
pub(crate) fn initialize_gpu_quadtree(
    mut quadtrees: ResMut<PersistentComponents<GpuQuadtree>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    compute_pipelines: Res<TerrainComputePipelines>,
    mut terrain_query: Query<(Entity, &TerrainConfig)>,
) {
    for (entity, config) in terrain_query.iter_mut() {
        quadtrees.insert(
            entity,
            GpuQuadtree::new(config, &device, &queue, &compute_pipelines),
        );
    }
}

/// Extracts the new nodes updates for all [`GpuQuadtree`]s by copying them over from their
/// corresponding [`Quadtree`]s.
pub(crate) fn update_gpu_quadtree(
    mut render_world: ResMut<RenderWorld>,
    mut terrain_query: Query<(Entity, &mut Quadtree)>,
) {
    let mut gpu_quadtrees = render_world.resource_mut::<PersistentComponents<GpuQuadtree>>();

    for (entity, mut quadtree) in terrain_query.iter_mut() {
        let gpu_quadtree = match gpu_quadtrees.get_mut(&entity) {
            Some(gpu_quadtree) => gpu_quadtree,
            None => continue,
        };

        gpu_quadtree.node_updates.clear();
        mem::swap(&mut quadtree.node_updates, &mut gpu_quadtree.node_updates);
    }
}

/// Queues the [`NodeUpdate`]s generated this frame for the quadtree update pipeline,
/// by filling the node update buffers with them.
pub(crate) fn queue_quadtree_updates(
    queue: Res<RenderQueue>,
    mut gpu_quadtrees: ResMut<PersistentComponents<GpuQuadtree>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    for entity in terrain_query.iter() {
        let gpu_quadtree = gpu_quadtrees.get_mut(&entity).unwrap();

        queue.write_buffer(
            &gpu_quadtree.update_buffer,
            0,
            cast_slice(&gpu_quadtree.node_updates),
        );
    }
}
