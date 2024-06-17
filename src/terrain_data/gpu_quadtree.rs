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
use ndarray::{Array2, Array4};
use std::mem;

/// Stores the GPU representation of the [`Quadtree`] (array texture)
/// alongside the data to update it.
///
/// The data is synchronized each frame by copying it from the [`Quadtree`] to the texture.
#[derive(Component)]
pub struct GpuQuadtree {
    pub(crate) quadtree_buffer: StaticBuffer<()>,
    pub(crate) origins_buffer: StaticBuffer<()>,
    /// The current cpu quadtree data. This is synced each frame with the quadtree data.
    data: Array4<QuadtreeEntry>,
    origins: Array2<UVec2>,
}

impl GpuQuadtree {
    fn new(device: &RenderDevice, quadtree: &Quadtree) -> Self {
        let quadtree_buffer = StaticBuffer::empty_sized(
            None,
            device,
            (quadtree.data.len() * mem::size_of::<QuadtreeEntry>()) as BufferAddress,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );

        let origins_buffer = StaticBuffer::empty_sized(
            None,
            device,
            (quadtree.origins.len() * mem::size_of::<UVec2>()) as BufferAddress,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );

        Self {
            quadtree_buffer,
            origins_buffer,
            data: default(),
            origins: default(),
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
        for terrain in &terrain_query {
            for view in &view_query {
                let quadtree = quadtrees.get(&(terrain, view)).unwrap();

                gpu_quadtrees.insert((terrain, view), GpuQuadtree::new(&device, quadtree));
            }
        }
    }

    /// Extracts the current data from all [`Quadtree`]s into the corresponding [`GpuQuadtree`]s.
    pub(crate) fn extract(
        mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
        quadtrees: Extract<Res<TerrainViewComponents<Quadtree>>>,
        view_query: Extract<Query<Entity, With<TerrainView>>>,
        terrain_query: Extract<Query<Entity, With<Terrain>>>,
    ) {
        for terrain in &terrain_query {
            for view in &view_query {
                let quadtree = quadtrees.get(&(terrain, view)).unwrap();
                let gpu_quadtree = gpu_quadtrees.get_mut(&(terrain, view)).unwrap();

                gpu_quadtree.data = quadtree.data.clone();
                gpu_quadtree.origins = quadtree.origins.clone();
            }
        }
    }

    /// Prepares the quadtree data to be copied into the quadtree texture.
    pub(crate) fn prepare(
        queue: Res<RenderQueue>,
        mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
        view_query: Query<Entity, With<TerrainView>>,
        terrain_query: Query<Entity, With<Terrain>>,
    ) {
        for terrain in &terrain_query {
            for view in &view_query {
                let gpu_quadtree = gpu_quadtrees.get_mut(&(terrain, view)).unwrap();

                let data = cast_slice(gpu_quadtree.data.as_slice().unwrap());
                gpu_quadtree.quadtree_buffer.update_bytes(&queue, data);

                let origins = cast_slice(gpu_quadtree.origins.as_slice().unwrap());
                gpu_quadtree.origins_buffer.update_bytes(&queue, origins);
            }
        }
    }
}
