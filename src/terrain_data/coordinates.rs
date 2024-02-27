use bevy::{prelude::*, render::render_resource::ShaderType};
use bincode::{Decode, Encode};
use std::{fmt};

#[cfg(feature = "spherical")]
fn sphere_to_cube(xy: Vec2) -> Vec2 {
    Vec2::new(if xy.x > 0.0 { 0.5 * (1.0 + 3.0 * xy.x).sqrt() } else { 1.0 - 0.5 * (1.0 - 3.0 * xy.x).sqrt() },
              if xy.y > 0.0 { 0.5 * (1.0 + 3.0 * xy.y).sqrt() } else { 1.0 - 0.5 * (1.0 - 3.0 * xy.y).sqrt() })
}

#[cfg(feature = "spherical")]
fn cube_to_sphere(uv: Vec2) -> Vec2 {
    Vec2::new(if uv.x > 0.5 { (4.0 * uv.x.powi(2) - 1.0) / 3.0 } else { (1.0 - 4.0 * (1.0 - uv.x).powi(2)) / 3.0 },
              if uv.y > 0.5 { (4.0 * uv.y.powi(2) - 1.0) / 3.0 } else { (1.0 - 4.0 * (1.0 - uv.y).powi(2)) / 3.0 })
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

    pub fn children(self) -> impl Iterator<Item=Self> {
        (0..4).map(move |index| {
            NodeCoordinate::new(
                self.side,
                self.lod + 1,
                (self.x << 1) + index % 2,
                (self.y << 1) + index / 2,
            )
        })
    }

    pub fn neighbours(self) -> impl Iterator<Item=Self> {
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

            self.neighbour_coordinate(neighbour_position)
        })
    }

    pub fn path(self, path: &str, extension: &str) -> String {
        format!("{path}/{self}.{extension}")
    }


    fn neighbour_coordinate(self, neighbour_position: IVec2) -> Self {
        let node_count = Self::node_count(self.lod) as i32;

        #[cfg(feature = "spherical")] {
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
        }

        #[cfg(not(feature = "spherical"))] {
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
}

impl fmt::Display for NodeCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}_{}_{}_{}", self.side, self.lod, self.x, self.y)
    }
}


#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) struct UVCoordinate {
    pub(crate) side: u32,
    pub(crate) uv: Vec2,
}

impl UVCoordinate {
    pub(crate) fn from_node_coordinate(node_coordinate: NodeCoordinate, node_count: u32) -> Self {
        let uv = (Vec2::new(
            node_coordinate.x as f32 + 0.5,
            node_coordinate.y as f32 + 0.5,
        )) / node_count as f32;

        Self {
            side: node_coordinate.side,
            uv,
        }
    }

    pub(crate) fn node_coordinate(&self, lod: u32) -> Vec2 {
        let node_count = NodeCoordinate::node_count(lod) as f32;
        let max_coordinate = Vec2::splat(node_count - 0.00001);

        (self.uv * node_count).clamp(Vec2::ZERO, max_coordinate)
    }

    pub(crate) fn from_local_position(local_position: Vec3) -> Self {
        #[cfg(feature = "spherical")]
        {
            let normal = local_position.normalize();
            let abs_normal = normal.abs();

            let (side, xy) = if abs_normal.x > abs_normal.y && abs_normal.x > abs_normal.z {
                if normal.x < 0.0 {
                    (0, Vec2::new(-normal.z, normal.y) / normal.x)
                } else {
                    (3, Vec2::new(-normal.y, normal.z) / normal.x)
                }
            } else if abs_normal.z > abs_normal.y {
                if normal.z > 0.0 {
                    (1, Vec2::new(normal.x, -normal.y) / normal.z)
                } else {
                    (4, Vec2::new(normal.y, -normal.x) / normal.z)
                }
            } else {
                if normal.y > 0.0 {
                    (2, Vec2::new(normal.x, normal.z) / normal.y)
                } else {
                    (5, Vec2::new(-normal.z, -normal.x) / normal.y)
                }
            };

            let uv = sphere_to_cube(xy);

            Self { side, uv }
        }

        #[cfg(not(feature = "spherical"))]
        return Self {
            side: 0,
            uv: Vec2::new(0.5 * local_position.x + 0.5, 0.5 * local_position.z + 0.5),
        };
    }

    pub(crate) fn local_position(self) -> Vec3 {
        #[cfg(feature = "spherical")]
        {
            let uv = cube_to_sphere(self.uv);

            match self.side {
                0 => Vec3::new(-1.0, -uv.y, uv.x),
                1 => Vec3::new(uv.x, -uv.y, 1.0),
                2 => Vec3::new(uv.x, 1.0, uv.y),
                3 => Vec3::new(1.0, -uv.x, uv.y),
                4 => Vec3::new(uv.y, -uv.x, -1.0),
                5 => Vec3::new(uv.y, -1.0, uv.x),
                _ => unreachable!(),
            }
                .normalize()
        }

        #[cfg(not(feature = "spherical"))]
        return Vec3::new(2.0 * self.uv.x - 1.0, 0.0, 2.0 * self.uv.y - 1.0);
    }

    #[cfg(feature = "spherical")]
    pub(crate) fn project_to_side(self, side: u32) -> Self {
        let info = SideInfo::project_to_side(self.side, side);

        let uv = info
            .map(|info| match info {
                SideInfo::Fixed0 => 0.0,
                SideInfo::Fixed1 => 1.0,
                SideInfo::PositiveS => self.uv.x,
                SideInfo::PositiveT => self.uv.y,
            })
            .into();

        Self { side, uv }
    }

    #[cfg(not(feature = "spherical"))]
    pub(crate) fn project_to_side(self, _side: u32) -> Self { self }
}

#[cfg(feature = "spherical")]
#[derive(Clone, Copy)]
enum SideInfo {
    Fixed0,
    Fixed1,
    PositiveS,
    PositiveT,
}

#[cfg(feature = "spherical")]
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
