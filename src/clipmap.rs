use crate::{
    descriptors::TileHierarchyDescriptor,
    pieces::{Piece, PieceVariant, StripeVariant, TriangleVariant},
};
use bevy::{prelude::*, render::render_resource::PrimitiveTopology};
use std::collections::HashMap;

#[derive(Component)]
pub struct Viewer;

#[derive(Component)]
pub struct TileHierarchy {
    meshes: HashMap<PieceVariant, Handle<Mesh>>,
}

impl TileHierarchy {
    pub fn new(meshes: &mut Assets<Mesh>) -> Self {
        let mesh = Mesh::new(PrimitiveTopology::TriangleList);

        Self {
            meshes: HashMap::from([
                (
                    PieceVariant::Triangle(TriangleVariant::Dense),
                    meshes.add(mesh.clone()),
                ),
                (
                    PieceVariant::Triangle(TriangleVariant::Sparse),
                    meshes.add(mesh.clone()),
                ),
                (
                    PieceVariant::Stripe(StripeVariant::Dense),
                    meshes.add(mesh.clone()),
                ),
                (
                    PieceVariant::Stripe(StripeVariant::Half),
                    meshes.add(mesh.clone()),
                ),
                (
                    PieceVariant::Stripe(StripeVariant::Sparse),
                    meshes.add(mesh.clone()),
                ),
            ]),
        }
    }

    pub fn get_mesh(&self, variant: PieceVariant) -> Handle<Mesh> {
        self.meshes
            .get(&variant)
            .unwrap_or_else(|| panic!("Clipmap is missing a mesh of type {:?}.", variant))
            .as_weak()
    }
}

pub fn update_hierarchy_on_change(
    mut meshes: ResMut<Assets<Mesh>>,
    hierarchy_query: Query<
        (&TileHierarchy, &TileHierarchyDescriptor),
        Changed<TileHierarchyDescriptor>,
    >,
) {
    for (hierarchy, hierarchy_descriptor) in hierarchy_query.iter() {
        for (&variant, mesh) in hierarchy.meshes.iter() {
            let mesh = meshes.get_mut(mesh).expect("Invalid mesh handle.");
            *mesh = Piece::new(
                hierarchy_descriptor.tile_size,
                hierarchy_descriptor.wireframe,
                variant,
            )
            .to_mesh();
        }
    }
}
