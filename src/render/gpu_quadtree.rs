use crate::{
    config::{NodeId, TerrainConfig},
    node_atlas::NodeAtlas,
    quadtree::{NodeUpdate, Quadtree},
    render::{layouts::NODE_UPDATE_SIZE, InitTerrain, PersistentComponent},
    TerrainComputePipelines,
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
use image::EncodableLayout;
use std::{mem, num::NonZeroU32};

/// Stores the GPU representation of the [`Quadtree`] alongside the data to update it.
pub struct GpuQuadtree {
    /// Readonly view of the entire quadtree, with all of its layers.
    pub(crate) view: TextureView,
    /// The node id, the node update buffer and its bind group for each layer of the quadtree
    /// used during the quadtree update pipeline.
    pub(crate) update: Vec<(NodeId, Buffer, BindGroup)>,
    /// All newly generate updates of nodes, which were activated or deactivated.
    /// (Taken from the [`Quadtree`] each frame.)
    node_updates: Vec<Vec<NodeUpdate>>,
}

impl GpuQuadtree {
    fn new(
        config: &TerrainConfig,
        device: &RenderDevice,
        queue: &RenderQueue,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Self {
        let (quadtree, view) = Self::create_quadtree(config, device, queue);
        let update = Self::create_quadtree_update(quadtree, config, device, compute_pipelines);

        Self {
            view,
            update,
            node_updates: default(),
        }
    }

    /// Creates the quadtree texture and its readonly view.
    fn create_quadtree(
        config: &TerrainConfig,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) -> (Texture, TextureView) {
        let data = vec![NodeAtlas::INACTIVE_ID; config.node_count as usize];

        let quadtree = device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: "quadtree_texture".into(),
                size: Extent3d {
                    width: config.chunk_count.x,
                    height: config.chunk_count.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: config.lod_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R16Uint,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            },
            data.as_bytes(),
        );

        let view = quadtree.create_view(&TextureViewDescriptor {
            label: "quadtree_view".into(),
            format: Some(TextureFormat::R16Uint),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        (quadtree, view)
    }

    /// Creates the [`NodeUpdate`] buffers, views and bind groups for each layer of the quadtree.
    fn create_quadtree_update(
        quadtree: Texture,
        config: &TerrainConfig,
        device: &RenderDevice,
        compute_pipelines: &TerrainComputePipelines,
    ) -> Vec<(u32, Buffer, BindGroup)> {
        (0..config.lod_count)
            .map(|lod| {
                let node_count = config.node_count(lod);
                let max_node_count = (node_count.x * node_count.y) as BufferAddress;

                let buffer = device.create_buffer(&BufferDescriptor {
                    label: "quadtree_update_buffer".into(),
                    size: NODE_UPDATE_SIZE * max_node_count,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let view = quadtree.create_view(&TextureViewDescriptor {
                    label: "quadtree_update_view".into(),
                    format: Some(TextureFormat::R16Uint),
                    dimension: Some(TextureViewDimension::D2),
                    aspect: TextureAspect::All,
                    base_mip_level: lod,
                    mip_level_count: NonZeroU32::new(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                });

                let bind_group = device.create_bind_group(&BindGroupDescriptor {
                    label: "quadtree_update_bind_group".into(),
                    layout: &compute_pipelines.update_quadtree_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: buffer.as_entire_binding(),
                        },
                    ],
                });

                (0, buffer, bind_group)
            })
            .collect()
    }

    fn extract(&mut self, quadtree: &mut Quadtree) {
        self.node_updates = mem::replace(
            &mut quadtree.node_updates,
            vec![default(); self.update.len()],
        );
    }
}

/// Initializes the [`GpuQuadtree`] of newly created terrains. Runs during the [`Prepare`](bevy::render::RenderStage::Prepare) stage.
pub(crate) fn init_gpu_quadtree(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    compute_pipelines: Res<TerrainComputePipelines>,
    mut gpu_quadtrees: ResMut<PersistentComponent<GpuQuadtree>>,
    terrain_query: Query<(Entity, &TerrainConfig), With<InitTerrain>>,
) {
    for (entity, config) in terrain_query.iter() {
        gpu_quadtrees.insert(
            entity,
            GpuQuadtree::new(config, &device, &queue, &compute_pipelines),
        );
    }
}

/// Extracts the [`NodeUpdate`]s generated this frame from the [`Quadtree`] to the [`GpuQuadtree`].
pub(crate) fn extract_quadtree(
    mut render_world: ResMut<RenderWorld>,
    mut terrain_query: Query<(Entity, &mut Quadtree), With<TerrainConfig>>,
) {
    let mut gpu_quadtrees = render_world.resource_mut::<PersistentComponent<GpuQuadtree>>();

    for (entity, mut quadtree) in terrain_query.iter_mut() {
        let gpu_quadtree = match gpu_quadtrees.get_mut(&entity) {
            Some(gpu_quadtree) => gpu_quadtree,
            None => continue,
        };

        // Todo: consider factoring this out as a trait of PersistentComponents similar to extract component
        gpu_quadtree.extract(&mut quadtree);
    }
}

/// Queues the [`NodeUpdate`]s generated this frame for the quadtree update pipeline,
/// by filling the node update buffers with them.
pub(crate) fn queue_quadtree_updates(
    queue: Res<RenderQueue>,
    mut gpu_quadtrees: ResMut<PersistentComponent<GpuQuadtree>>,
    terrain_query: Query<Entity, With<TerrainConfig>>,
) {
    for entity in terrain_query.iter() {
        let gpu_quadtree = gpu_quadtrees.get_mut(&entity).unwrap();

        for ((count, buffer, _), node_updates) in gpu_quadtree
            .update
            .iter_mut()
            .zip(&gpu_quadtree.node_updates)
        {
            queue.write_buffer(buffer, 0, cast_slice(node_updates));
            *count = node_updates.len() as u32;
        }
    }
}
