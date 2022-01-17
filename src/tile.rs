use bevy::{prelude::*, render::mesh::Indices, render::render_resource::PrimitiveTopology};
use itertools::iproduct;

#[inline]
fn add_quad(indices: &mut Vec<u16>, a: u16, b: u16, c: u16, d: u16, wireframe: bool) {
    if wireframe {
        indices.push(a);
        indices.push(b);
        indices.push(b);
        indices.push(c);
        indices.push(c);
        indices.push(a);
        indices.push(a);
        indices.push(d);
        indices.push(d);
        indices.push(c);
    } else {
        indices.push(a);
        indices.push(b);
        indices.push(c);
        indices.push(a);
        indices.push(c);
        indices.push(d);
    }
}

fn generate_positions(size: u8) -> Vec<[f32; 3]> {
    iproduct!(0..=size, 0..=size)
        .map(|(x, y)| [x as f32 / size as f32, 0.0, y as f32 / size as f32])
        .collect()
}

fn generate_indices(size: u16, wireframe: bool) -> Vec<u16> {
    let mut indices: Vec<u16> = Vec::new();

    for i in iproduct!(0..size, 0..size).map(|(x, y)| x + y * (size + 1)) {
        add_quad(
            &mut indices,
            i,
            i + 1,
            i + size + 2,
            i + size + 1,
            wireframe,
        );
    }

    indices
}

pub struct Tile {
    wireframe: bool,
    indices: Vec<u16>,
    positions: Vec<[f32; 3]>,
    // Todo: replace with two i16
    // positions: Vec<[i16; 2]>,
}

impl Tile {
    pub fn new(size: u8, wireframe: bool) -> Self {
        Self {
            wireframe,
            indices: generate_indices(size as u16, wireframe),
            positions: generate_positions(size),
        }
    }

    pub fn to_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(if self.wireframe {
            PrimitiveTopology::LineList
        } else {
            PrimitiveTopology::TriangleList
        });

        let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; self.positions.len()];
        let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; self.positions.len()];

        // set the attributes of the mesh
        mesh.set_indices(Some(Indices::U16(self.indices)));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);

        // Todo: remove once custom vertex attributes are implemented
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        mesh
    }
}
