use crate::math::{coordinate::Coordinate, SideInfo};
use bevy::{
    math::{DVec2, IVec2},
    render::render_resource::ShaderType,
};
use bincode::{Decode, Encode};
use std::fmt;

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

    pub fn neighbours(self) -> impl Iterator<Item = Self> {
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

        #[cfg(feature = "spherical")]
        {
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

        #[cfg(not(feature = "spherical"))]
        {
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

    pub(crate) fn to_coordinate(self, node_count: u32) -> Coordinate {
        let st = (DVec2::new(self.x as f64 + 0.5, self.y as f64 + 0.5)) / node_count as f64;

        Coordinate {
            side: self.side,
            st,
        }
    }
}

impl fmt::Display for NodeCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}_{}_{}_{}", self.side, self.lod, self.x, self.y)
    }
}
