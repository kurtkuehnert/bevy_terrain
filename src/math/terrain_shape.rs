use crate::math::spheroid::project_point_spheroid;
use bevy::{
    math::{DMat3, DVec3},
    prelude::*,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum TerrainShape {
    Plane { side_length: f64 },
    Sphere { radius: f64 },
    Spheroid { major_axis: f64, minor_axis: f64 },
}

impl TerrainShape {
    pub const WGS84: Self = TerrainShape::Spheroid {
        major_axis: 6378137.0,
        minor_axis: 6356752.314245,
    };

    fn diagonal(self) -> DVec3 {
        match self {
            TerrainShape::Plane { side_length } => DVec3::new(side_length, 1.0, side_length),
            TerrainShape::Sphere { radius } => DVec3::splat(radius),
            TerrainShape::Spheroid {
                major_axis,
                minor_axis,
            } => DVec3::new(major_axis, minor_axis, major_axis),
        }
    }

    pub fn transform(self) -> Transform {
        Transform::from_scale(self.diagonal().as_vec3())
    }
    pub fn local_from_unit(self) -> DMat3 {
        DMat3::from_diagonal(self.diagonal())
    }

    pub(crate) fn is_spherical(self) -> bool {
        match self {
            TerrainShape::Plane { .. } => false,
            TerrainShape::Sphere { .. } => true,
            TerrainShape::Spheroid { .. } => true,
        }
    }
    pub fn face_count(self) -> u32 {
        if self.is_spherical() {
            6
        } else {
            1
        }
    }
    pub fn scale(self) -> f64 {
        match self {
            TerrainShape::Plane { side_length } => side_length / 2.0,
            TerrainShape::Sphere { radius } => radius,
            TerrainShape::Spheroid {
                major_axis,
                minor_axis,
            } => (major_axis + minor_axis) / 2.0, // consider using major axis to be conservative
        }
    }

    pub fn position_unit_to_local(self, unit_position: DVec3, height: f64) -> DVec3 {
        let local_from_unit = self.local_from_unit();

        let local_position = local_from_unit * (unit_position);
        let local_normal = (local_from_unit
            * if self.is_spherical() {
                unit_position
            } else {
                DVec3::Y
            })
        .normalize();

        local_position + height * local_normal
    }

    pub fn position_local_to_unit(self, local_position: DVec3) -> DVec3 {
        let unit_from_local = self.local_from_unit().inverse();

        match self {
            TerrainShape::Plane { .. } => {
                DVec3::new(1.0, 0.0, 1.0) * (unit_from_local * local_position)
            }
            TerrainShape::Sphere { .. } => (unit_from_local * local_position).normalize(),
            TerrainShape::Spheroid {
                major_axis,
                minor_axis,
            } => {
                let surface_position =
                    project_point_spheroid(major_axis, minor_axis, local_position);
                (unit_from_local * surface_position).normalize()
            }
        }
    }
}
