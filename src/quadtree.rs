use crate::{
    descriptors::QuadtreeDescriptor,
    pipeline::{TerrainData, TileData},
    tile::Tile,
};
use bevy::{
    prelude::*,
    render::primitives::{Aabb, Frustum, Sphere},
};
use bevy_inspector_egui::Inspectable;
use itertools::iproduct;
use std::mem::take;

/// Marks a camera as the viewer of the terrain.
/// This is used to select the visible nodes of the quadtree.
/// The view distance is a multiplier, which increases the size of the lod ranges.
#[derive(Component, Inspectable)]
pub struct Viewer {
    #[inspectable(min = 1.0)]
    pub view_distance: f32,
}

impl Default for Viewer {
    fn default() -> Self {
        Self { view_distance: 2.0 }
    }
}

/// Selection of visible nodes.
/// They are divided into dense and sparse nodes.
/// The later have half the resolution of the former.
#[derive(Default)]
struct Selection {
    dense_nodes: Vec<TileData>,
    sparse_nodes: Vec<TileData>,
}

impl Selection {
    fn add_node(&mut self, node: Node, dense: bool) {
        if dense {
            self.dense_nodes.push(node.into());
        } else {
            self.sparse_nodes.push(node.into());
        }
    }
}

/// Node representation used while traversing the quadtree.
/// They get converted into [`TileData`], the Gpu equivalent.
#[derive(Clone, Debug)]
struct Node {
    position: UVec2,
    size: u32,
    lod: u8,
    range: f32,
}

impl From<Node> for TileData {
    fn from(val: Node) -> Self {
        TileData {
            position: val.position,
            size: val.size,
            range: val.range,
            color: Vec4::from(match val.lod {
                0 => Color::YELLOW,
                1 => Color::RED,
                2 => Color::GREEN,
                3 => Color::BLUE,
                4 => Color::YELLOW,
                5 => Color::RED,
                6 => Color::GREEN,
                7 => Color::BLUE,
                _ => Color::BLACK,
            }),
        }
    }
}

impl Node {
    /// Recursively traverses the quadtree, while selecting all currently visible nodes.
    fn select_lod(
        self,
        quadtree: &Quadtree,
        camera_position: Vec3,
        local_to_world: &Mat4,
        frustum: &Frustum,
        selection: &mut Selection,
    ) -> bool {
        // Todo: accurately calculate min and max heights
        let aabb = Aabb::from_min_max(
            Vec3::new(self.position.x as f32, 0.0, self.position.y as f32),
            Vec3::new(
                (self.position.x + self.size) as f32,
                500.0,
                (self.position.y + self.size) as f32,
            ),
        );

        let sphere = Sphere {
            center: camera_position,
            radius: self.range,
        };

        // test, whether the node is inside the current lod range
        if !sphere.intersects_obb(&aabb, local_to_world) {
            return false; // area handled by parent
        }

        // test, whether the node is inside the cameras view frustum
        if !frustum.intersects_obb(&aabb, local_to_world) {
            return true; // area not visible
        }

        // last lod can't be subdivided, so we included it altogether
        if self.lod == 0 {
            selection.add_node(self, true);
            return true; // area covered by the current node, which can't be subdivided
        }

        let lod = self.lod - 1;
        let range = quadtree.lod_ranges[lod as usize];
        let sphere = Sphere {
            center: camera_position,
            radius: range,
        };

        // test, whether the node is inside the next smaller lod range
        if !sphere.intersects_obb(&aabb, local_to_world) {
            selection.add_node(self, true);
            return true; // area covered by the current node, which isn't subdivided
        }

        let size = self.size >> 1;

        // if this loop is reached the node has to be subdivided into four children
        for (x, y) in iproduct!(0..2, 0..2) {
            let child = Node {
                position: self.position + UVec2::new(x * size, y * size),
                size,
                lod,
                range,
            };

            // test, whether the child is successfully selected
            if !child.clone().select_lod(
                quadtree,
                camera_position,
                local_to_world,
                frustum,
                selection,
            ) {
                // the child was not selected yet, so it has to be selected as a part of its parent,
                // with a sparse grid
                selection.add_node(child, false);
            }
        }

        true // area covered by the child nodes
    }
}

/// The quadtree responsible for the [`Node`] selection.
/// Every frame the tree is traversed and all visible nodes are selected.
#[derive(Component, Inspectable)]
pub struct Quadtree {
    node_size: u32,
    lod_count: u8,
    lod_ranges: Vec<f32>,
}

impl Default for Quadtree {
    fn default() -> Self {
        Self {
            node_size: 4,
            lod_count: 4,
            lod_ranges: Vec::new(),
        }
    }
}

pub fn traverse_quadtree(
    viewer_query: Query<(&GlobalTransform, &Frustum), (With<Viewer>, With<Camera>)>,
    mut quadtree_query: Query<(&Children, &Quadtree, &GlobalTransform)>,
    mut terrain_query: Query<&mut TerrainData>,
) {
    for (children, quadtree, terrain_transform) in quadtree_query.iter_mut() {
        for (camera_transform, frustum) in viewer_query.iter() {
            let lod = quadtree.lod_count - 1;

            let root = Node {
                position: UVec2::ZERO,
                size: quadtree.node_size * (1 << lod as u32),
                lod,
                range: quadtree.lod_ranges[lod as usize],
            };

            let mut selection = Selection::default();

            root.select_lod(
                quadtree,
                camera_transform.translation,
                &terrain_transform.compute_matrix(),
                frustum,
                &mut selection,
            );

            // println!(
            //     "{} {}",
            //     selection.dense_nodes.len(),
            //     selection.sparse_nodes.len()
            // );

            for &child in children.iter() {
                let mut instance_data = terrain_query.get_mut(child).unwrap();

                if instance_data.sparse {
                    instance_data.data = take(&mut selection.sparse_nodes);
                } else {
                    instance_data.data = take(&mut selection.dense_nodes);
                }
            }
        }
    }
}

pub fn update_quadtree_on_change(
    mut meshes: ResMut<Assets<Mesh>>,
    viewer_query: Query<&Viewer>,
    mut quadtree_query: Query<
        (&Children, &mut Quadtree, &QuadtreeDescriptor),
        Changed<QuadtreeDescriptor>,
    >,
    terrain_query: Query<(&TerrainData, &Handle<Mesh>)>,
) {
    for (children, mut quadtree, quadtree_descriptor) in quadtree_query.iter_mut() {
        quadtree.node_size = quadtree_descriptor.node_size as u32;
        quadtree.lod_count = quadtree_descriptor.lod_count;
        update_view_distance(viewer_query.iter(), &mut quadtree);

        for &child in children.iter() {
            let (terrain_data, mesh) = terrain_query.get(child).unwrap();

            let mesh = meshes
                .get_mut(mesh.clone())
                .expect("Instance mesh not initialized.");

            let size = if terrain_data.sparse {
                quadtree.node_size / 2
            } else {
                quadtree.node_size
            } as u8;

            *mesh = Tile::new(size, quadtree_descriptor.wireframe).to_mesh();
        }
    }
}

pub fn update_view_distance_on_change(
    mut quadtree_query: Query<&mut Quadtree>,
    viewer_query: Query<&Viewer, Changed<Viewer>>,
) {
    for mut quadtree in quadtree_query.iter_mut() {
        update_view_distance(viewer_query.iter(), &mut quadtree);
    }
}

fn update_view_distance<'a>(viewer: impl Iterator<Item = &'a Viewer>, quadtree: &mut Quadtree) {
    for viewer in viewer {
        quadtree.lod_ranges = (0..quadtree.lod_count)
            .map(|lod| quadtree.node_size as f32 * viewer.view_distance * (2 << lod as u32) as f32)
            .collect();

        // println!("{:?}", quadtree.lod_ranges);
    }
}
