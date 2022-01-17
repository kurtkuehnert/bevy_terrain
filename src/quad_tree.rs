use crate::{
    descriptors::QuadtreeDescriptor,
    material::{InstanceData, TileData},
    tile::Tile,
};
use bevy::{
    prelude::*,
    render::primitives::{Aabb, Frustum, Sphere},
};
use bevy_inspector_egui::Inspectable;
use itertools::iproduct;

fn intersect(sphere: &Sphere, aabb: &Aabb) -> bool {
    // get box closest point to sphere center by clamping
    let x = (aabb.center.x - aabb.half_extents.x)
        .max(sphere.center.x.min(aabb.center.x + aabb.half_extents.x));
    let y = (aabb.center.y - aabb.half_extents.y)
        .max(sphere.center.y.min(aabb.center.y + aabb.half_extents.y));

    let z = (aabb.center.z - aabb.half_extents.z)
        .max(sphere.center.z.min(aabb.center.z + aabb.half_extents.z));

    let distance = (x - sphere.center.x) * (x - sphere.center.x)
        + (y - sphere.center.y) * (y - sphere.center.y)
        + (z - sphere.center.z) * (z - sphere.center.z);

    distance < sphere.radius.powf(2.0)
}

#[derive(Component, Inspectable)]
pub struct Viewer {
    #[inspectable(min = 0.0)]
    pub view_distance: f32,
}

impl Default for Viewer {
    fn default() -> Self {
        Self {
            view_distance: 32.0 * 2.0_f32.powf(8.0),
        }
    }
}

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
    fn select_lod(
        self,
        quadtree: &Quadtree,
        camera_position: Vec3,
        frustum: &Frustum,
        selection: &mut Selection,
    ) -> bool {
        let model = Mat4::IDENTITY;
        // Todo: accurately calculate min and max heights
        let aabb = Aabb::from_min_max(
            Vec3::new(self.position.x as f32, -1000.0, self.position.y as f32),
            Vec3::new(
                (self.position.x + self.size) as f32,
                1000.0,
                (self.position.y + self.size) as f32,
            ),
        );
        let sphere = Sphere {
            center: camera_position,
            radius: self.range,
        };

        // test, whether the node is inside the current lod range
        if !intersect(&sphere, &aabb) {
            return false; // area handled by parent
        }

        // test, whether the node is inside the cameras view frustum
        if !frustum.intersects_obb(&aabb, &model) {
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
        if !sphere.intersects_obb(&aabb, &model) {
            selection.add_node(self, true);
            return true; // area covered by the current node, which isn't subdivided
        }

        // if this loop is reached the node has to be subdivided into four children
        for (x, y) in iproduct!(0..2, 0..2) {
            let size = self.size >> 1;

            let child = Node {
                position: self.position + UVec2::new(x * size, y * size),
                size,
                lod,
                range,
            };

            // test, whether the child is successfully selected
            if !child
                .clone()
                .select_lod(quadtree, camera_position, frustum, selection)
            {
                // the child was not selected yet, so it has to be selected as a part of its parent,
                // with a sparse grid
                selection.add_node(child, false);
            }
        }

        true // area covered by the child nodes
    }
}

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
    mut terrain_query: Query<(&Children, &Quadtree)>,
    mut instance_query: Query<&mut InstanceData>,
) {
    for (children, quadtree) in terrain_query.iter_mut() {
        for (transform, frustum) in viewer_query.iter() {
            let lod = quadtree.lod_count - 1;

            let root = Node {
                position: UVec2::ZERO,
                size: quadtree.node_size * (1 << quadtree.lod_count as u32),
                lod,
                range: quadtree.lod_ranges[lod as usize],
            };

            let mut selection = Selection::default();

            root.select_lod(quadtree, transform.translation, frustum, &mut selection);

            let Selection {
                dense_nodes,
                sparse_nodes,
            } = selection;

            println!("{} {}", dense_nodes.len(), sparse_nodes.len());
            //
            // // println!("{:?} {:?}", dense_nodes, sparse_nodes);
            // println!("{:?}", quadtree.lod_ranges);
            //
            // for node in sparse_nodes.iter().chain(dense_nodes.iter()) {
            //     println!("{:?}", node);
            // }

            for &child in children.iter() {
                let mut instance_data = instance_query.get_mut(child).unwrap();

                if instance_data.sparse {
                    instance_data.instance_data = sparse_nodes.clone();
                } else {
                    instance_data.instance_data = dense_nodes.clone();
                }
            }
        }
    }
}

pub fn update_quadtree_on_change(
    mut meshes: ResMut<Assets<Mesh>>,
    viewer_query: Query<&Viewer>,
    mut terrain_query: Query<
        (&Children, &mut Quadtree, &QuadtreeDescriptor),
        Changed<QuadtreeDescriptor>,
    >,
    mut instance_query: Query<(&mut InstanceData, &Handle<Mesh>)>,
) {
    for (children, mut quadtree, quadtree_descriptor) in terrain_query.iter_mut() {
        quadtree.node_size = quadtree_descriptor.node_size as u32;
        quadtree.lod_count = quadtree_descriptor.lod_count;
        update_view_distance(viewer_query.iter(), &mut quadtree);

        for &child in children.iter() {
            let (mut instance_data, mesh) = instance_query.get_mut(child).unwrap();

            instance_data.wireframe = quadtree_descriptor.wireframe;
            let mesh = meshes
                .get_mut(mesh.clone())
                .expect("Instance mesh not initialized.");

            let size = if instance_data.sparse {
                quadtree.node_size / 2
            } else {
                quadtree.node_size
            } as u8;

            *mesh = Tile::new(size, instance_data.wireframe).to_mesh();
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
            .rev()
            .map(|lod| viewer.view_distance / (1 << lod as u32) as f32)
            .collect();
    }
}
