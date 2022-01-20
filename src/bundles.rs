use crate::{
    descriptors::QuadtreeDescriptor, material::TerrainMaterial, pipeline::TerrainData,
    quadtree::Quadtree,
};
use bevy::{prelude::*, render::primitives::Aabb, render::render_resource::PrimitiveTopology};

#[derive(Bundle)]
pub struct InstanceBundle {
    mesh: Handle<Mesh>,
    material: Handle<TerrainMaterial>,
    instance_data: TerrainData,
    transform: Transform,
    global_transform: GlobalTransform,
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
    aabb: Aabb,
}

impl InstanceBundle {
    pub fn new(meshes: &mut Assets<Mesh>, material: Handle<TerrainMaterial>, sparse: bool) -> Self {
        Self {
            mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
            material,
            instance_data: TerrainData {
                sparse,
                ..Default::default()
            },
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            aabb: Aabb::from_min_max(Vec3::splat(-10000.0), Vec3::splat(10000.0)),
        }
    }
}

#[derive(Default, Bundle)]
pub struct TerrainBundle {
    pub quadtree: Quadtree,
    pub quadtree_descriptor: QuadtreeDescriptor,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}
