mod coordinate;
mod ellipsoid;
mod surface_approximation;
mod terrain_model;

pub use crate::math::{
    coordinate::{Coordinate, TileCoordinate},
    surface_approximation::SurfaceApproximation,
    terrain_model::TerrainModel,
};

/// The square of the parameter c of the algebraic sigmoid function, used to convert between uv and st coordinates.
const C_SQR: f64 = 0.87 * 0.87;
