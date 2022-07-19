use crate::{
    quadtree::{Quadtree, QuadtreeEntry},
    terrain::Terrain,
    terrain_view::TerrainView,
    TerrainViewComponents,
};
use bevy::{
    core::cast_slice,
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        Extract,
    },
};
use ndarray::Array3;
use std::num::NonZeroU32;

/// Stores the GPU representation of the [`Quadtree`] alongside the data to update it.
#[derive(Component)]
pub struct GpuQuadtree {
    lod_count: u32,
    node_count: u32,
    quadtree_texture: Texture, // Todo: consider image handle
    pub(crate) quadtree_view: TextureView,
    pub(crate) data: Array3<QuadtreeEntry>, // Todo: consider own component
}

impl GpuQuadtree {
    fn new(device: &RenderDevice, quadtree: &Quadtree) -> Self {
        let quadtree_texture = device.create_texture(&TextureDescriptor {
            label: "quadtree_texture".into(),
            size: Extent3d {
                width: quadtree.node_count,
                height: quadtree.node_count,
                depth_or_array_layers: quadtree.lod_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rg16Uint,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        });

        let quadtree_view = quadtree_texture.create_view(&TextureViewDescriptor::default());

        Self {
            lod_count: quadtree.lod_count,
            node_count: quadtree.node_count,
            quadtree_texture,
            quadtree_view,
            data: default(),
        }
    }

    fn update(&self, queue: &RenderQueue) {
        queue.write_texture(
            ImageCopyTexture {
                texture: &self.quadtree_texture,
                mip_level: 0,
                origin: Origin3d { x: 0, y: 0, z: 0 },
                aspect: TextureAspect::All,
            },
            cast_slice(&self.data.as_slice().unwrap()),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(self.node_count * 4),
                rows_per_image: NonZeroU32::new(self.node_count),
            },
            Extent3d {
                width: self.node_count,
                height: self.node_count,
                depth_or_array_layers: self.lod_count,
            },
        );
    }
}

/// Initializes the [`GpuQuadtree`] of newly created terrains.
pub(crate) fn initialize_gpu_quadtree(
    device: Res<RenderDevice>,
    mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
    quadtrees: Extract<Res<TerrainViewComponents<Quadtree>>>,
    view_query: Extract<Query<Entity, With<TerrainView>>>,
    terrain_query: Extract<Query<Entity, Added<Terrain>>>,
) {
    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            let quadtree = quadtrees.get(&(terrain, view)).unwrap();

            gpu_quadtrees.insert((terrain, view), GpuQuadtree::new(&device, &quadtree));
        }
    }
}

/// Extracts the new nodes updates for all [`GpuQuadtree`]s by copying them over from their
/// corresponding [`Quadtree`]s.
pub(crate) fn update_gpu_quadtree(
    mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
    quadtrees: Extract<Res<TerrainViewComponents<Quadtree>>>,
    view_query: Extract<Query<Entity, With<TerrainView>>>,
    terrain_query: Extract<Query<Entity, With<Terrain>>>,
) {
    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            if let Some((quadtree, gpu_quadtree)) = quadtrees
                .get(&(terrain, view))
                .zip(gpu_quadtrees.get_mut(&(terrain, view)))
            {
                // Todo: enable this again once mutable access to the main world in extract is less painful
                // mem::swap(&mut gpu_quadtree.data, &mut gpu_gpu_quadtree.data);
                gpu_quadtree.data = quadtree.data.clone();
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
            let gpu_quadtree = gpu_quadtrees.get_mut(&(terrain, view)).unwrap();
            gpu_quadtree.update(&queue);
        }
    }
}
