use crate::{
    terrain::Terrain,
    terrain_data::quadtree::{Quadtree, QuadtreeEntry},
    terrain_view::{TerrainView, TerrainViewComponents},
    util::StaticBuffer,
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
use ndarray::Array4;
use std::{iter, mem};

/// Stores the GPU representation of the [`Quadtree`] (array texture)
/// alongside the data to update it.
///
/// The data is synchronized each frame by copying it from the [`Quadtree`] to the texture.
#[derive(Component)]
pub struct GpuQuadtree {
    pub(crate) quadtree_buffer: StaticBuffer<()>,
    /// The current cpu quadtree data. This is synced each frame with the quadtree data.
    data: Array4<QuadtreeEntry>,
}

impl GpuQuadtree {
    fn new(device: &RenderDevice, quadtree: &Quadtree) -> Self {
        let quadtree_buffer = StaticBuffer::empty_sized(
            device,
            (quadtree.data.len() * mem::size_of::<QuadtreeEntry>()) as BufferAddress,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );

        Self {
            quadtree_buffer,
            data: default(),
        }
    }

    /// Initializes the [`GpuQuadtree`] of newly created terrains.
    pub(crate) fn initialize(
        device: Res<RenderDevice>,
        mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
        quadtrees: Extract<Res<TerrainViewComponents<Quadtree>>>,
        view_query: Extract<Query<Entity, With<TerrainView>>>,
        terrain_query: Extract<Query<Entity, Added<Terrain>>>,
    ) {
        for terrain_view in iter::zip(&terrain_query, &view_query) {
            let quadtree = quadtrees.get(&terrain_view).unwrap();

            gpu_quadtrees.insert(terrain_view, GpuQuadtree::new(&device, quadtree));
        }
    }

    /// Extracts the current data from all [`Quadtree`]s into the corresponding [`GpuQuadtree`]s.
    pub(crate) fn extract(
        mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
        quadtrees: Extract<Res<TerrainViewComponents<Quadtree>>>,
        view_query: Extract<Query<Entity, With<TerrainView>>>,
        terrain_query: Extract<Query<Entity, With<Terrain>>>,
    ) {
        for terrain_view in iter::zip(&terrain_query, &view_query) {
            let quadtree = quadtrees.get(&terrain_view).unwrap();
            let gpu_quadtree = gpu_quadtrees.get_mut(&terrain_view).unwrap();

            gpu_quadtree.data = quadtree.data.clone();
        }
    }

    /// Prepares the quadtree data to be copied into the quadtree texture.
    pub(crate) fn prepare(
        queue: Res<RenderQueue>,
        mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
        view_query: Query<Entity, With<TerrainView>>,
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        for terrain_view in iter::zip(&terrain_query, &view_query) {
            let gpu_quadtree = gpu_quadtrees.get_mut(&terrain_view).unwrap();

            let data = cast_slice(gpu_quadtree.data.as_slice().unwrap());
            gpu_quadtree.quadtree_buffer.update_bytes(&queue, data);
        }
    }
}
