use crate::math::{cube_to_sphere, sphere_to_cube, SideInfo};
use bevy::math::{DVec2, DVec3};

/// Describes a location on the unit cube sphere.
/// The side index refers to one of the six cube faces and the st coordinate describes the location within this side.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Coordinate {
    pub(crate) side: u32,
    pub(crate) st: DVec2,
}

impl Coordinate {
    /// Calculates the coordinate for for the local position on the unit cube sphere.
    pub(crate) fn from_local_position(local_position: DVec3) -> Self {
        #[cfg(feature = "spherical")]
        {
            let normal = local_position.normalize();
            let abs_normal = normal.abs();

            let (side, uv) = if abs_normal.x > abs_normal.y && abs_normal.x > abs_normal.z {
                if normal.x < 0.0 {
                    (0, DVec2::new(-normal.z / normal.x, normal.y / normal.x))
                } else {
                    (3, DVec2::new(-normal.y / normal.x, normal.z / normal.x))
                }
            } else if abs_normal.z > abs_normal.y {
                if normal.z > 0.0 {
                    (1, DVec2::new(normal.x / normal.z, -normal.y / normal.z))
                } else {
                    (4, DVec2::new(normal.y / normal.z, -normal.x / normal.z))
                }
            } else {
                if normal.y > 0.0 {
                    (2, DVec2::new(normal.x / normal.y, normal.z / normal.y))
                } else {
                    (5, DVec2::new(-normal.z / normal.y, -normal.x / normal.y))
                }
            };

            let st = cube_to_sphere(uv);

            Self { side, st }
        }

        #[cfg(not(feature = "spherical"))]
        return Self {
            side: 0,
            st: DVec2::new(0.5 * local_position.x + 0.5, 0.5 * local_position.z + 0.5),
        };
    }

    pub(crate) fn local_position(self) -> DVec3 {
        #[cfg(feature = "spherical")]
        {
            let uv = sphere_to_cube(self.st);

            match self.side {
                0 => DVec3::new(-1.0, -uv.y, uv.x),
                1 => DVec3::new(uv.x, -uv.y, 1.0),
                2 => DVec3::new(uv.x, 1.0, uv.y),
                3 => DVec3::new(1.0, -uv.x, uv.y),
                4 => DVec3::new(uv.y, -uv.x, -1.0),
                5 => DVec3::new(uv.y, -1.0, uv.x),
                _ => unreachable!(),
            }
            .normalize()
        }

        #[cfg(not(feature = "spherical"))]
        return DVec3::new(2.0 * self.st.x - 1.0, 0.0, 2.0 * self.st.y - 1.0);
    }

    /// Projects the coordinate onto one of the six cube faces.
    /// Thereby it chooses the closest location on this face to the original coordinate.
    pub(crate) fn project_to_side(self, side: u32) -> Self {
        #[cfg(feature = "spherical")]
        {
            let info = SideInfo::project_to_side(self.side, side);

            let st = info
                .map(|info| match info {
                    SideInfo::Fixed0 => 0.0,
                    SideInfo::Fixed1 => 1.0,
                    SideInfo::PositiveS => self.st.x,
                    SideInfo::PositiveT => self.st.y,
                })
                .into();

            Self { side, st }
        }

        #[cfg(not(feature = "spherical"))]
        self
    }

    pub(crate) fn node_coordinate(&self, lod: u32) -> DVec2 {
        let node_count = (1 << lod) as f64;
        let max_coordinate = DVec2::splat(node_count - 0.00001);

        (self.st * node_count).clamp(DVec2::ZERO, max_coordinate)
    }
}
