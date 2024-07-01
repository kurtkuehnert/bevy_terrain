use crate::math::{TerrainModel, C_SQR};
use bevy::{
    math::{DVec2, DVec3, IVec2},
    render::render_resource::ShaderType,
};
use bincode::{Decode, Encode};
use std::fmt;

fn sphere_to_cube(st: DVec2) -> DVec2 {
    let w = (st - 0.5) / 0.5;
    w / (1.0 + C_SQR - C_SQR * w * w).powf(0.5)
}

/// Converts uv coordinates in range [-1,1] to st coordinates in range [0,1].
/// The uv coordinates are spaced equally on the surface of the cube and
/// the st coordinates are spaced equally on the surface of the sphere.
fn cube_to_sphere(uv: DVec2) -> DVec2 {
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

/// Describes a location on the unit cube sphere.
/// The side index refers to one of the six cube faces and the st coordinate describes the location within this side.
#[derive(Copy, Clone, Debug, Default)]
pub struct Coordinate {
    pub side: u32,
    pub st: DVec2,
}

impl Coordinate {
    /// Calculates the coordinate for for the local position on the unit cube sphere.
    pub(crate) fn from_world_position(world_position: DVec3, model: &TerrainModel) -> Self {
        let local_position = model.position_world_to_local(world_position);

        if model.spherical {
            let normal = local_position;
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
        } else {
            Self {
                side: 0,
                st: DVec2::new(local_position.x + 0.5, local_position.z + 0.5)
                    .clamp(DVec2::ZERO, DVec2::ONE),
            }
        }
    }

    pub(crate) fn world_position(self, model: &TerrainModel) -> DVec3 {
        let normal = if model.spherical {
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
        } else {
            DVec3::new(2.0 * self.st.x - 1.0, 0.0, 2.0 * self.st.y - 1.0)
        };

        model.position_local_to_world(normal)
    }

    /// Projects the coordinate onto one of the six cube faces.
    /// Thereby it chooses the closest location on this face to the original coordinate.
    pub(crate) fn project_to_side(self, side: u32, model: &TerrainModel) -> Self {
        if model.spherical {
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
        } else {
            self
        }
    }
}

/// The global coordinate and identifier of a node.
#[derive(Copy, Clone, Default, Debug, Hash, Eq, PartialEq, ShaderType, Encode, Decode)]
pub struct NodeCoordinate {
    /// The side of the cube sphere the node is located on.
    pub side: u32,
    /// The lod of the node, where 0 is the highest level of detail with the smallest size
    /// and highest resolution
    pub lod: u32,
    /// The x position of the node in node sizes.
    pub x: u32,
    /// The y position of the node in node sizes.
    pub y: u32,
}

impl NodeCoordinate {
    pub const INVALID: NodeCoordinate = NodeCoordinate {
        side: u32::MAX,
        lod: u32::MAX,
        x: u32::MAX,
        y: u32::MAX,
    };

    pub fn new(side: u32, lod: u32, x: u32, y: u32) -> Self {
        Self { side, lod, x, y }
    }

    pub fn node_count(lod: u32) -> u32 {
        1 << lod
    }

    pub fn parent(self) -> Self {
        Self {
            side: self.side,
            lod: self.lod.wrapping_sub(1),
            x: self.x >> 1,
            y: self.y >> 1,
        }
    }

    pub fn children(self) -> impl Iterator<Item = Self> {
        (0..4).map(move |index| {
            NodeCoordinate::new(
                self.side,
                self.lod + 1,
                (self.x << 1) + index % 2,
                (self.y << 1) + index / 2,
            )
        })
    }

    pub fn neighbours(self, spherical: bool) -> impl Iterator<Item = Self> {
        const OFFSETS: [IVec2; 8] = [
            IVec2::new(0, -1),
            IVec2::new(1, 0),
            IVec2::new(0, 1),
            IVec2::new(-1, 0),
            IVec2::new(-1, -1),
            IVec2::new(1, -1),
            IVec2::new(1, 1),
            IVec2::new(-1, 1),
        ];

        OFFSETS.iter().map(move |&offset| {
            let neighbour_position = IVec2::new(self.x as i32, self.y as i32) + offset;

            self.neighbour_coordinate(neighbour_position, spherical)
        })
    }

    pub fn path(self, path: &str, extension: &str) -> String {
        format!("{path}/{self}.{extension}")
    }

    fn neighbour_coordinate(self, neighbour_position: IVec2, spherical: bool) -> Self {
        let node_count = Self::node_count(self.lod) as i32;

        if spherical {
            let edge_index = match neighbour_position {
                IVec2 { x, y }
                    if x < 0 && y < 0
                        || x < 0 && y >= node_count
                        || x >= node_count && y < 0
                        || x >= node_count && y >= node_count =>
                {
                    return Self::INVALID;
                }
                IVec2 { x, .. } if x < 0 => 1,
                IVec2 { y, .. } if y < 0 => 2,
                IVec2 { x, .. } if x >= node_count => 3,
                IVec2 { y, .. } if y >= node_count => 4,
                _ => 0,
            };

            let neighbour_position = neighbour_position
                .clamp(IVec2::ZERO, IVec2::splat(node_count - 1))
                .as_uvec2();

            let neighbouring_sides = [
                [0, 4, 2, 1, 5],
                [1, 0, 2, 3, 5],
                [2, 0, 4, 3, 1],
                [3, 2, 4, 5, 1],
                [4, 2, 0, 5, 3],
                [5, 4, 0, 1, 3],
            ];

            let neighbour_side = neighbouring_sides[self.side as usize][edge_index];

            let info = SideInfo::project_to_side(self.side, neighbour_side);

            let [x, y] = info.map(|info| match info {
                SideInfo::Fixed0 => 0,
                SideInfo::Fixed1 => node_count as u32 - 1,
                SideInfo::PositiveS => neighbour_position.x,
                SideInfo::PositiveT => neighbour_position.y,
            });

            Self::new(neighbour_side, self.lod, x, y)
        } else {
            if neighbour_position.x < 0
                || neighbour_position.y < 0
                || neighbour_position.x >= node_count
                || neighbour_position.y >= node_count
            {
                Self::INVALID
            } else {
                Self::new(
                    self.side,
                    self.lod,
                    neighbour_position.x as u32,
                    neighbour_position.y as u32,
                )
            }
        }
    }

    pub(crate) fn world_position(self, model: &TerrainModel) -> DVec3 {
        let st = (DVec2::new(self.x as f64 + 0.5, self.y as f64 + 0.5))
            / Self::node_count(self.lod) as f64;

        let coordinate = Coordinate {
            side: self.side,
            st,
        };

        coordinate.world_position(model)
    }
}

impl fmt::Display for NodeCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}_{}_{}_{}", self.side, self.lod, self.x, self.y)
    }
}
