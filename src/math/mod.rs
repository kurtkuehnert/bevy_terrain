mod coordinate;
mod ellipsoid;
#[cfg(feature = "high_precision")]
mod surface_approximation;
mod terrain_model;

use bevy::math::DMat3;

#[cfg(feature = "high_precision")]
pub use crate::math::surface_approximation::SurfaceApproximation;
pub use crate::math::{
    coordinate::{Coordinate, TileCoordinate, ViewCoordinate},
    terrain_model::TerrainModel,
};

/// The square of the parameter c of the algebraic sigmoid function, used to convert between uv and st coordinates.
const C_SQR: f64 = 0.87 * 0.87;

/// One matrix per side, which shuffles the a, b, and c component to their corresponding position.
const SIDE_MATRICES: [DMat3; 6] = [
    DMat3::from_cols_array(&[-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, -1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[0.0, 0.0, -1.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0]),
    DMat3::from_cols_array(&[0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0]),
];

/// Inverse/Transpose of `SIDE_MATRICES`.
const INVERSE_SIDE_MATRICES: [DMat3; 6] = [
    DMat3::from_cols_array(&[-1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[0.0, 0.0, 1.0, 0.0, -1.0, 0.0, -1.0, 0.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
];
