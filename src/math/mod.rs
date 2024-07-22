mod coordinate;
mod ellipsoid;
#[cfg(feature = "high_precision")]
mod surface_approximation;
mod terrain_model;

pub use crate::math::{
    coordinate::{Coordinate, TileCoordinate, ViewCoordinate},
    terrain_model::TerrainModel,
};

#[cfg(feature = "high_precision")]
pub use crate::math::surface_approximation::SurfaceApproximation;

/// The square of the parameter c of the algebraic sigmoid function, used to convert between uv and st coordinates.
const C_SQR: f64 = 0.87 * 0.87;
