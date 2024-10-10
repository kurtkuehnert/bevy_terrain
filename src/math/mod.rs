mod coordinate;
mod ellipsoid;
#[cfg(feature = "high_precision")]
mod surface_approximation;
mod terrain_model;

use bevy::math::{DMat3, DVec2, IVec2};

#[cfg(feature = "high_precision")]
pub use crate::math::surface_approximation::SurfaceApproximation;
pub use crate::math::{
    coordinate::{Coordinate, TileCoordinate, ViewCoordinate},
    terrain_model::TerrainModel,
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

#[derive(Clone, Copy)]
pub enum FaceRotation {
    Identical,
    ClockWise,
    CounterClockWise,
    Backside,
}

impl FaceRotation {
    const EVEN_LIST: [FaceRotation; 6] = [
        FaceRotation::Identical,
        FaceRotation::Identical,
        FaceRotation::CounterClockWise,
        FaceRotation::Backside,
        FaceRotation::ClockWise,
        FaceRotation::Identical,
    ];

    const ODD_LIST: [FaceRotation; 6] = [
        FaceRotation::Identical,
        FaceRotation::Identical,
        FaceRotation::ClockWise,
        FaceRotation::Backside,
        FaceRotation::CounterClockWise,
        FaceRotation::Identical,
    ];

    fn face_rotation(face: u32, other_face: u32) -> Self {
        let index = ((6 + other_face - face) % 6) as usize;

        if face % 2 == 0 {
            FaceRotation::EVEN_LIST[index]
        } else {
            FaceRotation::ODD_LIST[index]
        }
    }
}

#[derive(Clone, Copy)]
enum FaceProjection {
    Fixed0,
    Fixed1,
    PositiveU,
    PositiveV,
}

impl FaceProjection {
    const EVEN_LIST: [[FaceProjection; 2]; 6] = [
        [FaceProjection::PositiveU, FaceProjection::PositiveV],
        [FaceProjection::Fixed0, FaceProjection::PositiveV],
        [FaceProjection::Fixed0, FaceProjection::PositiveU],
        [FaceProjection::PositiveV, FaceProjection::PositiveU],
        [FaceProjection::PositiveV, FaceProjection::Fixed0],
        [FaceProjection::PositiveU, FaceProjection::Fixed0],
    ];
    const ODD_LIST: [[FaceProjection; 2]; 6] = [
        [FaceProjection::PositiveU, FaceProjection::PositiveV],
        [FaceProjection::PositiveU, FaceProjection::Fixed1],
        [FaceProjection::PositiveV, FaceProjection::Fixed1],
        [FaceProjection::PositiveV, FaceProjection::PositiveU],
        [FaceProjection::Fixed1, FaceProjection::PositiveU],
        [FaceProjection::Fixed1, FaceProjection::PositiveV],
    ];

    fn project_to_face(face: u32, other_face: u32, uv: DVec2) -> DVec2 {
        let index = ((6 + other_face - face) % 6) as usize;

        let info = if face % 2 == 0 {
            FaceProjection::EVEN_LIST[index]
        } else {
            FaceProjection::ODD_LIST[index]
        };

        info.map(|info| match info {
            FaceProjection::Fixed0 => 0.0,
            FaceProjection::Fixed1 => 1.0,
            FaceProjection::PositiveU => uv.x,
            FaceProjection::PositiveV => uv.y,
        })
        .into()
    }
}
