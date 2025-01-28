use crate::math::ellipsoid::project_point_ellipsoid;
use bevy::{
    math::{DMat4, DVec3},
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

    pub fn transform(self) -> Transform {
        Transform::from_matrix(self.local_from_unit().as_mat4())
    }
    pub fn local_from_unit(self) -> DMat4 {
        match self {
            TerrainShape::Plane { side_length } => {
                DMat4::from_scale(DVec3::new(side_length, 1.0, side_length))
            }
            TerrainShape::Sphere { radius } => DMat4::from_scale(DVec3::splat(radius)),
            TerrainShape::Spheroid {
                major_axis,
                minor_axis,
            } => DMat4::from_scale(DVec3::new(major_axis, minor_axis, major_axis)),
        }
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

        let local_position = local_from_unit.transform_point3(unit_position);
        let local_normal = local_from_unit
            .transform_vector3(if self.is_spherical() {
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
                DVec3::new(1.0, 0.0, 1.0) * unit_from_local.transform_point3(local_position)
            }
            TerrainShape::Sphere { .. } => {
                unit_from_local.transform_point3(local_position).normalize()
            }
            TerrainShape::Spheroid {
                major_axis,
                minor_axis,
            } => {
                let surface_position = project_point_ellipsoid(
                    DVec3::new(major_axis, major_axis, minor_axis),
                    local_position,
                );
                unit_from_local
                    .transform_point3(surface_position)
                    .normalize()
            }
        }
    }
}
