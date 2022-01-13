use bevy::{prelude::*, render::mesh::Indices, render::render_resource::PrimitiveTopology};

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum TriangleVariant {
    Dense,
    Sparse,
}

impl Default for TriangleVariant {
    fn default() -> Self {
        Self::Dense
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum StripeVariant {
    Dense,
    Half,
    Sparse,
}

impl Default for StripeVariant {
    fn default() -> Self {
        Self::Dense
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash, Component)]
pub enum PieceVariant {
    Triangle(TriangleVariant),
    Stripe(StripeVariant),
}

impl Default for PieceVariant {
    fn default() -> Self {
        Self::Triangle(Default::default())
    }
}

#[inline]
fn add_triangle(indices: &mut Vec<u16>, a: u16, b: u16, c: u16, wireframe: bool) {
    if wireframe {
        indices.push(a);
        indices.push(b);
        indices.push(b);
        indices.push(c);
        indices.push(c);
        indices.push(a);
    } else {
        indices.push(a);
        indices.push(b);
        indices.push(c);
    }
}

#[inline]
fn add_split_triangle(
    indices: &mut Vec<u16>,
    a: u16,
    b: u16,
    c: u16,
    d: u16,
    split: bool,
    wireframe: bool,
) {
    if split {
        add_triangle(indices, a, b, d, wireframe);
        add_triangle(indices, b, c, d, wireframe);
    } else {
        add_triangle(indices, a, b, c, wireframe);
    }
}

fn generate_triangle_indices(size: u16, variant: TriangleVariant, wireframe: bool) -> Vec<u16> {
    let mut indices: Vec<u16> = Vec::new();

    let dense = variant == TriangleVariant::Dense; // is dense
    let start_dense = (size + 1).pow(2); // first dense index (9)

    for line in 0..size {
        let start_sparse = line.pow(2); // first sparse index of the line
        let d = (line + 1) * 2; // distance to the next sparse vertex above

        for i in start_sparse..start_sparse + d - 1 {
            let j = start_dense + line + i; // dense index

            if i % 2 == line % 2 {
                // triangle pointing down (0-1-2, 0-2-3, 1-4-5, 1-5-6, 2-5-6, 2-6-7, 3-6-7, 3-7-8)
                add_split_triangle(&mut indices, i + d - 1, i + d, i, j, dense, wireframe);
                add_split_triangle(&mut indices, i, i + d, i + d + 1, j + 1, dense, wireframe);
            } else {
                // triangle pointing up (1-6-2, 2-6-3)
                add_split_triangle(&mut indices, i + d, i, i - 1, j, dense, wireframe);
                add_split_triangle(&mut indices, i + 1, i, i + d, j + 1, dense, wireframe);
            }
        }
    }

    indices
}

fn generate_triangle_positions(size: u16, variant: TriangleVariant) -> Vec<[f32; 3]> {
    // sparse vertices (0,1,2,3,4,5,6,7,8)
    let positions = (0..=size as i16)
        .flat_map(|y| (-y..=y).map(move |x| [(2 * x) as f32, 0.0, (2 * y) as f32]));

    match variant {
        TriangleVariant::Dense => {
            // dense vertices (9,10,11,12,13,14)
            let dense_positions = (1..=size as i16).flat_map(|y| {
                (0..2 * y).map(move |x| [(2 * x - 2 * y + 1) as f32, 0.0, (2 * y - 1) as f32])
            });

            positions.chain(dense_positions).collect()
        }
        TriangleVariant::Sparse => positions.collect(),
    }
}

fn generate_stripe_indices(size: u16, variant: StripeVariant, wireframe: bool) -> Vec<u16> {
    let mut indices: Vec<u16> = Vec::new();

    // amount of dense vertices per line
    let offset_dense = match variant {
        StripeVariant::Dense => 3,
        StripeVariant::Half => 2,
        StripeVariant::Sparse => 0,
    };

    let dense = variant == StripeVariant::Dense; // is dense
    let sparse = variant != StripeVariant::Sparse; // is not sparse

    let start_dense = 3 * size + 1; // first dense index (7)

    for line in 0..size {
        let i = 3 * line; // sparse index
        let j = start_dense + offset_dense * line; // dense index

        if line != size - 1 {
            // outer triangles (3-4-1, 3-2-5)
            add_split_triangle(&mut indices, i + 1, i + 3, i + 4, j + 1, sparse, wireframe);
            add_split_triangle(&mut indices, i + 5, i + 3, i + 2, j + 2, dense, wireframe);
        }

        // inner triangles (0-2-1, 1-2-3, 3-5-4, 4-5-6)
        add_split_triangle(&mut indices, i + 1, i, i + 2, j, sparse, wireframe);
        add_split_triangle(&mut indices, i + 2, i + 3, i + 1, j, sparse, wireframe);
    }

    indices
}

fn generate_stripe_positions(size: u16, variant: StripeVariant) -> Vec<[f32; 3]> {
    let mut positions = Vec::new();

    // sparse vertices (0,1,2,3,4,5)
    for line in 0..size {
        let pos = (2 * line) as f32;
        positions.push([pos, 0.0, pos]);
        positions.push([pos + 2.0, 0.0, pos]);
        positions.push([pos, 0.0, pos + 2.0]);
    }

    // top right sparse vertex (6)
    positions.push([(2 * size) as f32, 0.0, (2 * size) as f32]);

    if variant != StripeVariant::Sparse {
        for line in 0..size - 1 {
            let pos = (2 * line) as f32;
            // dense vertices (7,8)
            positions.push([pos + 1.0, 0.0, pos + 1.0]);
            positions.push([pos + 3.0, 0.0, pos + 1.0]);

            if variant == StripeVariant::Dense {
                // dense vertices (9)
                positions.push([pos + 1.0, 0.0, pos + 3.0]);
            }
        }

        // top right dense vertex (10)
        positions.push([(2 * size - 1) as f32, 0.0, (2 * size - 1) as f32]);
    }

    positions
}

/// Generates the indices and positions for the triangle of the specified size and variant.
///  4   4-------5-------6-------7-------8   
///        \   / | \   / | \   / | \   /    
///  3       11  |   12  |   13  |   14    
///            \ | /   \ | /   \ | /    
///  2           1-------2-------3           line=1  d=4  start_sparse=1
///                \   / | \   /          
///  1               9   |   10          
///                    \ | /
///  0                   0                   line=0  d=2  start_sparse=0
///
/// y/x -4  -3  -2  -1   0   1   2   3   4
///
/// start_dense=9
///
/// 0,1,2,3,4,5,6,7,8 - sparse indices
/// 9,10,11,12,13,14  - dense indices
/// The maximum size is 180, because this generates 65341 vertices, which is the last size smaller
/// than the u16 limit.
pub fn generate_triangle(
    size: u16,
    variant: TriangleVariant,
    wireframe: bool,
) -> (Vec<u16>, Vec<[f32; 3]>) {
    (
        generate_triangle_indices(size - 1, variant, wireframe),
        generate_triangle_positions(size - 1, variant),
    )
}

/// Generates the indices and positions for the stripe of the specified size and variant.
///  4          5-------6      
///           / | \     |
///  3      9   |   10  |
///       /     |     \ |
///  2  2-------3-------4      line=1  i=3  j=10
///     | \     |     /
///  1  |   7   |   8
///     |     \ | /
///  0  0-------1              line=0  i=0  j=7
///
/// y/x 0   1   2   3   4
///
/// start_dense=7
///
/// 0,1,2,3,4,5,6 - sparse indices
/// 7,8,9,10      - dense indices
/// 7,8,10        - only included for half and full stripes
/// 9             - only included for full stripes
pub fn generate_stripe(
    size: u16,
    variant: StripeVariant,
    wireframe: bool,
) -> (Vec<u16>, Vec<[f32; 3]>) {
    (
        generate_stripe_indices(size, variant, wireframe),
        generate_stripe_positions(size, variant),
    )
}

pub struct Piece {
    wireframe: bool,
    indices: Vec<u16>,
    positions: Vec<[f32; 3]>,
    // Todo: replace with two i16
    // positions: Vec<[i16; 2]>,
}

impl Piece {
    pub fn new(size: u8, wireframe: bool, variant: PieceVariant) -> Self {
        let (indices, positions) = match variant {
            PieceVariant::Triangle(variant) => generate_triangle(size as u16, variant, wireframe),
            PieceVariant::Stripe(variant) => generate_stripe(size as u16, variant, wireframe),
        };

        Self {
            wireframe,
            indices,
            positions,
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
