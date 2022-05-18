use crate::{
    config::TerrainConfig,
    quadtree::{NodeActivation, NodeDeactivation, Quadtree},
    render::{
        layouts::{NODE_ACTIVATION_SIZE, NODE_DEACTIVATION_SIZE},
        PersistentComponents,
    },
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
use itertools::iproduct;
use std::mem;

/// Stores the GPU representation of the [`Quadtree`] alongside the data to update it.
#[derive(Component)]
pub struct GpuQuadtree {
    pub(crate) view: TextureView,
    pub(crate) update_bind_group: BindGroup,
    pub(crate) activation_buffer: Buffer,
    pub(crate) deactivation_buffer: Buffer,
    pub(crate) activation_count: u32,
    pub(crate) deactivation_count: u32,
    pub(crate) node_activations: Vec<NodeActivation>,
    pub(crate) node_deactivations: Vec<NodeDeactivation>, // Todo: consider own component
}

impl GpuQuadtree {
    fn new(
        config: &TerrainConfig,
        device: &RenderDevice,
        queue: &RenderQueue,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Self {
        let (view, update_bind_group, activation_buffer, deactivation_buffer) =
            Self::create_quadtree(config, device, queue, compute_pipelines);

        Self {
            view,
            update_bind_group,
            activation_buffer,
            deactivation_buffer,
            activation_count: 0,
            deactivation_count: 0,
            node_activations: default(),
            node_deactivations: default(),
        }
    }

    fn create_quadtree(
        config: &TerrainConfig,
        device: &RenderDevice,
        queue: &RenderQueue,
        compute_pipelines: &TerrainComputePipelines,
    ) -> (TextureView, BindGroup, Buffer, Buffer) {
        let size = config.load_count * config.load_count * config.lod_count;
        let data = vec![u64::MAX; (size) as usize];

        let quadtree = device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: "quadtree_texture".into(),
                size: Extent3d {
                    width: config.load_count,
                    height: config.load_count,
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

        let activation_buffer = device.create_buffer(&BufferDescriptor {
            label: "quadtree_activation_buffer".into(),
            size: 10 * NODE_ACTIVATION_SIZE * size as BufferAddress,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let deactivation_buffer = device.create_buffer(&BufferDescriptor {
            label: "quadtree_deactivation_buffer".into(),
            size: 10 * NODE_DEACTIVATION_SIZE * size as BufferAddress,
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
                    resource: activation_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: deactivation_buffer.as_entire_binding(),
                },
            ],
        });

        (
            view,
            update_bind_group,
            activation_buffer,
            deactivation_buffer,
        )
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

        gpu_quadtree.node_activations = mem::take(&mut quadtree.node_activations);
        gpu_quadtree.node_deactivations = mem::take(&mut quadtree.node_deactivations);
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

        let mut activations = Vec::new();

        for node_activation in &gpu_quadtree.node_activations {
            let position = TerrainConfig::node_position(node_activation.node_id);

            for lod in 0..=position.lod {
                let scale = 1 << (position.lod - lod);

                let origin_x = position.x * scale;
                let origin_y = position.y * scale;

                for (x, y) in iproduct!(origin_x..origin_x + scale, origin_y..origin_y + scale) {
                    let node_id = TerrainConfig::node_id(lod, x, y);

                    activations.push(NodeActivation {
                        node_id,
                        atlas_index: node_activation.atlas_index,
                        lod: position.lod,
                    })
                }
            }
        }

        gpu_quadtree.activation_count = activations.len() as u32;
        queue.write_buffer(&gpu_quadtree.activation_buffer, 0, cast_slice(&activations));

        let mut deactivations = Vec::new();

        for node_deactivation in &gpu_quadtree.node_deactivations {
            let position = TerrainConfig::node_position(node_deactivation.node_id);

            let ancestor_id =
                TerrainConfig::node_id(position.lod + 1, position.x >> 1, position.y >> 1);

            for lod in 0..=position.lod {
                let scale = 1 << (position.lod - lod);

                let origin_x = position.x * scale;
                let origin_y = position.y * scale;

                for (x, y) in iproduct!(origin_x..origin_x + scale, origin_y..origin_y + scale) {
                    let node_id = TerrainConfig::node_id(lod, x, y);

                    deactivations.push(NodeDeactivation {
                        node_id,
                        ancestor_id,
                    })
                }
            }
        }

        gpu_quadtree.deactivation_count = deactivations.len() as u32;
        queue.write_buffer(
            &gpu_quadtree.deactivation_buffer,
            0,
            cast_slice(&deactivations),
        );
    }
}
