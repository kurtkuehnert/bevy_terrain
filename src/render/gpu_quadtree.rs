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
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
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
    handle: Handle<Image>,
    data: Array3<QuadtreeEntry>,
}

impl GpuQuadtree {
    const FORMAT: TextureFormat = TextureFormat::Rg16Uint;

    fn new(device: &RenderDevice, quadtree: &Quadtree, images: &mut RenderAssets<Image>) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: "quadtree_texture".into(),
            size: Extent3d {
                width: quadtree.node_count,
                height: quadtree.node_count,
                depth_or_array_layers: quadtree.lod_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        });

        images.insert(
            quadtree.handle.clone(),
            GpuImage {
                texture_view: texture.create_view(&TextureViewDescriptor::default()),
                texture,
                texture_format: Self::FORMAT,
                sampler: device.create_sampler(&SamplerDescriptor::default()),
                size: Vec2::splat(quadtree.node_count as f32),
            },
        );

        Self {
            lod_count: quadtree.lod_count,
            node_count: quadtree.node_count,
            handle: quadtree.handle.clone(),
            data: default(),
        }
    }

    fn update(&self, queue: &RenderQueue, images: &RenderAssets<Image>) {
        let image = images.get(&self.handle).unwrap();

        queue.write_texture(
            ImageCopyTexture {
                texture: &image.texture,
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
    mut images: ResMut<RenderAssets<Image>>,
    mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
    quadtrees: Extract<Res<TerrainViewComponents<Quadtree>>>,
    view_query: Extract<Query<Entity, With<TerrainView>>>,
    terrain_query: Extract<Query<Entity, Added<Terrain>>>,
) {
    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            let quadtree = quadtrees.get(&(terrain, view)).unwrap();

            gpu_quadtrees.insert(
                (terrain, view),
                GpuQuadtree::new(&device, &quadtree, &mut images),
            );
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
            let quadtree = quadtrees.get(&(terrain, view)).unwrap();
            let gpu_quadtree = gpu_quadtrees.get_mut(&(terrain, view)).unwrap();

            // Todo: enable this again once mutable access to the main world in extract is less painful
            // mem::swap(&mut gpu_quadtree.data, &mut gpu_gpu_quadtree.data);
            gpu_quadtree.data = quadtree.data.clone();
        }
    }
}

/// Queues the [`NodeUpdate`]s generated this frame for the quadtree update pipeline,
/// by filling the node update buffers with them.
pub(crate) fn queue_quadtree_updates(
    queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
    mut gpu_quadtrees: ResMut<TerrainViewComponents<GpuQuadtree>>,
    view_query: Query<Entity, With<TerrainView>>,
    terrain_query: Query<Entity, With<Terrain>>,
) {
    for terrain in terrain_query.iter() {
        for view in view_query.iter() {
            let gpu_quadtree = gpu_quadtrees.get_mut(&(terrain, view)).unwrap();
            gpu_quadtree.update(&queue, &images);
        }
    }
}
