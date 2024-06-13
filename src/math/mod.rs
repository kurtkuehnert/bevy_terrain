mod coordinate;
mod model_view_approximation;
mod node;

pub(crate) use crate::math::{
    coordinate::Coordinate,
    model_view_approximation::{generate_model_view_approximation, ModelViewApproximation},
    node::NodeCoordinate,
};

use bevy::math::DVec2;

/// The square of the parameter c of the algebraic sigmoid function, used to convert between uv and st coordinates.
const C_SQR: f64 = 0.87 * 0.87;

pub(crate) fn sphere_to_cube(st: DVec2) -> DVec2 {
    let w = (st - 0.5) / 0.5;
    w / (1.0 + C_SQR - C_SQR * w * w).powf(0.5)
}

/// Converts uv coordinates in range [-1,1] to st coordinates in range [0,1].
/// The uv coordinates are spaced equally on the surface of the cube and
/// the st coordinates are spaced equally on the surface of the sphere.
pub(crate) fn cube_to_sphere(uv: DVec2) -> DVec2 {
    let w = uv * ((1.0 + C_SQR) / (1.0 + C_SQR * uv * uv)).powf(0.5);
    0.5 * w + 0.5
}

#[derive(Clone, Copy)]
enum SideInfo {
    Fixed0,
    Fixed1,
    PositiveS,
    PositiveT,
}

impl SideInfo {
    const EVEN_LIST: [[SideInfo; 2]; 6] = [
        [SideInfo::PositiveS, SideInfo::PositiveT],
        [SideInfo::Fixed0, SideInfo::PositiveT],
        [SideInfo::Fixed0, SideInfo::PositiveS],
        [SideInfo::PositiveT, SideInfo::PositiveS],
        [SideInfo::PositiveT, SideInfo::Fixed0],
        [SideInfo::PositiveS, SideInfo::Fixed0],
    ];
    const ODD_LIST: [[SideInfo; 2]; 6] = [
        [SideInfo::PositiveS, SideInfo::PositiveT],
        [SideInfo::PositiveS, SideInfo::Fixed1],
        [SideInfo::PositiveT, SideInfo::Fixed1],
        [SideInfo::PositiveT, SideInfo::PositiveS],
        [SideInfo::Fixed1, SideInfo::PositiveS],
        [SideInfo::Fixed1, SideInfo::PositiveT],
    ];

    fn project_to_side(side: u32, other_side: u32) -> [SideInfo; 2] {
        let index = ((6 + other_side - side) % 6) as usize;

        if side % 2 == 0 {
            SideInfo::EVEN_LIST[index]
        } else {
            SideInfo::ODD_LIST[index]
        }
    }
}
