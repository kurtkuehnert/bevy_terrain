use crate::{
    terrain::Terrain,
    terrain_data::quadtree::{Quadtree, QuadtreeEntry},
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

/// Stores the GPU representation of the [`Quadtree`] (array texture)
/// alongside the data to update it.
///
/// The data is synchronized each frame by copying it from the [`Quadtree`] to the texture.
#[derive(Component)]
pub struct GpuQuadtree {
    /// The handle of the quadtree texture.
    handle: Handle<Image>,
    /// The current cpu quadtree data. This is synced each frame with the quadtree data.
    data: Array3<QuadtreeEntry>,
    /// The count of level of detail layers.
    lod_count: u32,
    /// The count of nodes in x and y direction per layer.
    node_count: u32,
}

impl GpuQuadtree {
    /// The format of the quadtree texture.
    /// * R - The atlas index of the node.
    /// * G - The lod of the node.
    const FORMAT: TextureFormat = TextureFormat::Rg16Uint;

    /// Creates a new gpu quadtree and initializes its texture.
    fn new(device: &RenderDevice, images: &mut RenderAssets<Image>, quadtree: &Quadtree) -> Self {
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
            view_formats: &[],
        });

        images.insert(
            quadtree.handle.clone(),
            GpuImage {
                texture_view: texture.create_view(&TextureViewDescriptor::default()),
                texture,
                texture_format: Self::FORMAT,
                sampler: device.create_sampler(&SamplerDescriptor::default()),
                size: Vec2::splat(quadtree.node_count as f32),
                mip_level_count: quadtree.lod_count,
            },
        );

        Self {
            handle: quadtree.handle.clone(),
            data: default(),
            lod_count: quadtree.lod_count,
            node_count: quadtree.node_count,
        }
    }

    /// Updates the quadtree texture with the current data.
    fn update(&self, queue: &RenderQueue, images: &RenderAssets<Image>) {
        let image = images.get(&self.handle).unwrap();

        queue.write_texture(
            ImageCopyTexture {
                texture: &image.texture,
                mip_level: 0,
                origin: Origin3d { x: 0, y: 0, z: 0 },
                aspect: TextureAspect::All,
            },
            cast_slice(self.data.as_slice().unwrap()),
            ImageDataLayout {
                offset: 0,
                bytes_per_row:  Some(self.node_count * 4) ,
                rows_per_image:  Some(self.node_count) ,
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

          //  let quadtree = quadtrees.get(&(terrain, view)).unwrap();

            match quadtrees.get(&(terrain, view)){
                Some(tree) => {
                    gpu_quadtrees.insert(
                        (terrain, view),
                        GpuQuadtree::new(&device, &mut images, tree),
                    );
                },
                None => {
                    println!("WARN: Could not initialize gpu quadtree");
                }
            }

            


        }
    }
}

/// Extracts the current data from all [`Quadtree`]s into the corresponding [`GpuQuadtree`]s.
pub(crate) fn extract_quadtree(
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

/// Prepares the quadtree data to be copied into the quadtree texture.
pub(crate) fn prepare_quadtree(
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
