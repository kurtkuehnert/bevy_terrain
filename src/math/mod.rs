mod coordinate;
mod ellipsoid;
#[cfg(feature = "high_precision")]
mod surface_approximation;
mod terrain_model;

use bevy::math::{DMat3, DVec2, IVec2};
use std::mem;

#[cfg(feature = "high_precision")]
pub use crate::math::surface_approximation::SurfaceApproximation;
pub use crate::math::{
    coordinate::{Coordinate, TileCoordinate, ViewCoordinate},
    terrain_model::{sync_terrain_position, TerrainModel},
};

/// The square of the parameter c of the algebraic sigmoid function, used to convert between uv and st coordinates.
const C_SQR: f64 = 0.87 * 0.87;

const BLOCK_SIZE: i32 = 8;

/// One matrix per face, which shuffles the a, b, and c component to their corresponding position.
const FACE_MATRICES: [DMat3; 6] = [
    DMat3::from_cols_array(&[-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, -1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[0.0, 0.0, -1.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0]),
    DMat3::from_cols_array(&[0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0]),
];

/// Inverse/Transpose of `SIDE_MATRICES`.
const INVERSE_FACE_MATRICES: [DMat3; 6] = [
    DMat3::from_cols_array(&[-1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[0.0, 0.0, 1.0, 0.0, -1.0, 0.0, -1.0, 0.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
];

const NEIGHBOUR_OFFSETS: [IVec2; 8] = [
    IVec2::new(0, -1),
    IVec2::new(1, 0),
    IVec2::new(0, 1),
    IVec2::new(-1, 0),
    IVec2::new(-1, -1),
    IVec2::new(1, -1),
    IVec2::new(1, 1),
    IVec2::new(-1, 1),
];

const NEIGHBOURING_FACES: [[u32; 5]; 6] = [
    [0, 2, 1, 5, 4],
    [1, 2, 3, 5, 0],
    [2, 4, 3, 1, 0],
    [3, 4, 5, 1, 2],
    [4, 0, 5, 3, 2],
    [5, 0, 1, 3, 4],
];

#[repr(u32)]
pub enum FaceRotation {
    Identical = 0, // i
    ShiftU = 1,    // x
    RotateCCW = 2, // l
    Backside = 3,  // b
    RotateCW = 4,  // r
    ShiftV = 5,    // y
}

impl FaceRotation {
    fn project_uv(self, coordinate: Coordinate) -> DVec2 {
        let DVec2 { x: u, y: v } = coordinate.uv;
        let odd = (coordinate.face % 2) as f64;

        match self {
            FaceRotation::Identical => DVec2::new(u, v),
            FaceRotation::ShiftU => DVec2::new(odd, v),
            FaceRotation::RotateCCW => DVec2::new(odd, u),
            FaceRotation::Backside => DVec2::new(v, u),
            FaceRotation::RotateCW => DVec2::new(v, odd),
            FaceRotation::ShiftV => DVec2::new(u, odd),
        }
    }

    fn new(face: u32, other_face: u32) -> Self {
        let index = if (face % 2) == 0 {
            (6 + other_face - face) % 6
        } else {
            (6 + face - other_face) % 6
        };

        // Safety: safe because index is mod 6 and we have 6 enum variants
        let face: FaceRotation = unsafe { mem::transmute(index) };

        face
    }
}
