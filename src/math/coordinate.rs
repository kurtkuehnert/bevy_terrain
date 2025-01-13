use crate::math::{
    FaceRotation, TerrainModel, BLOCK_SIZE, C_SQR, FACE_MATRICES, INVERSE_FACE_MATRICES,
    NEIGHBOURING_FACES, NEIGHBOUR_OFFSETS,
};
use bevy::{
    math::{DVec2, DVec3},
    prelude::*,
    render::render_resource::ShaderType,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
};

/// Describes a location on the unit cube sphere.
/// The face index refers to one of the six cube faces and the uv coordinate describes the location within this face.
#[derive(Copy, Clone, Debug, Default)]
pub struct Coordinate {
    pub face: u32,
    pub uv: DVec2,
}

impl Coordinate {
    pub fn new(face: u32, uv: DVec2) -> Self {
        Self { face, uv }
    }

    pub fn from_unit_position(unit_position: DVec3, is_spherical: bool) -> Self {
        if is_spherical {
            let face = match unit_position {
                DVec3 { x, y, z } if x.abs() > y.abs() && x.abs() > z.abs() && x < 0.0 => 0,
                DVec3 { x, y, z } if x.abs() > y.abs() && x.abs() > z.abs() => 3,
                DVec3 { y, z, .. } if z.abs() > y.abs() && z > 0.0 => 1,
                DVec3 { y, z, .. } if z.abs() > y.abs() => 4,
                DVec3 { y, .. } if y > 0.0 => 2,
                _ => 5,
            };

            let abc = INVERSE_FACE_MATRICES[face as usize] * unit_position;
            let xy = abc.yz() / abc.x;

            let uv = 0.5 * xy * ((1.0 + C_SQR) / (1.0 + C_SQR * xy * xy)).powf(0.5) + 0.5;

            Self { face, uv }
        } else {
            let uv = DVec2::new(unit_position.x + 0.5, unit_position.z + 0.5)
                .clamp(DVec2::ZERO, DVec2::ONE);

            Self { face: 0, uv }
        }
    }

    pub fn unit_position(self, is_spherical: bool) -> DVec3 {
        if is_spherical {
            let xy =
                (2.0 * self.uv - 1.0) / (1.0 - 4.0 * C_SQR * (self.uv - 1.0) * self.uv).powf(0.5);

            FACE_MATRICES[self.face as usize] * DVec3::new(1.0, xy.x, xy.y).normalize()
        } else {
            DVec3::new(self.uv.x - 0.5, 0.0, self.uv.y - 0.5)
        }
    }

    /// Calculates the coordinate for for the unit position on the unit cube sphere.
    pub fn from_local_position(world_position: DVec3, model: &TerrainModel) -> Self {
        let unit_position = model.position_local_to_unit(world_position);

        Self::from_unit_position(unit_position, model.is_spherical())
    }

    pub fn local_position(self, model: &TerrainModel, height: f32) -> DVec3 {
        let unit_position = self.unit_position(model.is_spherical());
        model.position_unit_to_local(unit_position, height as f64)
    }

    /// Projects the coordinate onto one of the six cube faces.
    /// Thereby it chooses the closest location on this face to the original coordinate.
    pub fn project_to_face(self, face: u32) -> Self {
        Self {
            face,
            uv: FaceRotation::new(self.face, face).project_uv(self),
        }
    }
}

/// The global coordinate and identifier of a tile.
#[derive(Copy, Clone, Default, Debug, Hash, Eq, PartialEq, ShaderType, Serialize, Deserialize)]
pub struct TileCoordinate {
    /// The face of the cube sphere the tile is located on.
    pub face: u32,
    /// The lod of the tile, where 0 is the highest level of detail with the smallest size
    /// and highest resolution
    pub lod: u32,
    /// The xy position of the tile in tile sizes.
    pub xy: IVec2,
}

impl TileCoordinate {
    pub const INVALID: TileCoordinate = TileCoordinate {
        face: u32::MAX,
        lod: u32::MAX,
        xy: IVec2::MAX,
    };

    pub fn new(face: u32, lod: u32, xy: IVec2) -> Self {
        Self { face, lod, xy }
    }

    pub fn count(lod: u32) -> u32 {
        1 << lod
    }

    pub fn path(self, path: &Path) -> PathBuf {
        let tile_block = self.xy / BLOCK_SIZE;

        path.join(format!(
            "{}/{}_{}/{}.tif",
            self.lod, tile_block.x, tile_block.y, self
        ))
    }

    pub fn parent(self) -> Option<Self> {
        self.lod.checked_sub(1).map(|lod| Self {
            face: self.face,
            lod,
            xy: self.xy >> 1,
        })
    }

    pub fn children(self) -> impl Iterator<Item = Self> {
        (0..4).map(move |index| {
            TileCoordinate::new(
                self.face,
                self.lod + 1,
                IVec2::new((self.xy.x << 1) + index % 2, (self.xy.y << 1) + index / 2),
            )
        })
    }

    pub fn neighbours(self, spherical: bool) -> impl Iterator<Item = (Self, FaceRotation)> {
        NEIGHBOUR_OFFSETS.iter().map(move |&offset| {
            let edge_position = self.xy + offset;

            let tile_count = Self::count(self.lod) as i32;
            let scale = (tile_count - 1) as f64;

            if spherical {
                let edge_uv = (edge_position.as_dvec2() / scale).clamp(DVec2::ZERO, DVec2::ONE);
                let edge_coordinate = Coordinate::new(self.face, edge_uv);
                let edge_index = match edge_position {
                    IVec2 { x, y }
                        if x < 0 && y < 0
                            || x < 0 && y >= tile_count
                            || x >= tile_count && y < 0
                            || x >= tile_count && y >= tile_count =>
                    {
                        return (Self::INVALID, FaceRotation::Identical); // there is no single neighbour for tiles at the corners
                    }
                    IVec2 { y, .. } if y < 0 => 1,           // up
                    IVec2 { x, .. } if x >= tile_count => 2, // right
                    IVec2 { y, .. } if y >= tile_count => 3, // down
                    IVec2 { x, .. } if x < 0 => 4,           // left
                    _ => 0,
                };

                let neighbour_face = NEIGHBOURING_FACES[self.face as usize][edge_index];
                let neighbour_coordinate = edge_coordinate.project_to_face(neighbour_face);
                let neighbour_xy = (neighbour_coordinate.uv * scale).as_ivec2();
                let rotation = FaceRotation::new(self.face, neighbour_face);

                (Self::new(neighbour_face, self.lod, neighbour_xy), rotation)
            } else if edge_position.x < 0
                || edge_position.y < 0
                || edge_position.x >= tile_count
                || edge_position.y >= tile_count
            {
                (Self::INVALID, FaceRotation::Identical)
            } else {
                (
                    Self::new(self.face, self.lod, edge_position),
                    FaceRotation::Identical,
                )
            }
        })
    }
}

impl fmt::Display for TileCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}_{}_{}_{}", self.face, self.lod, self.xy.x, self.xy.y)
    }
}

#[derive(Copy, Clone, Default, Debug, ShaderType)]
pub struct ViewCoordinate {
    pub xy: IVec2,
    pub uv: Vec2,
}

impl ViewCoordinate {
    pub fn new(coordinate: Coordinate, lod: u32) -> Self {
        let count = TileCoordinate::count(lod) as f64;

        Self {
            xy: (coordinate.uv * count).as_ivec2(),
            uv: (coordinate.uv * count).fract().as_vec2(),
        }
    }
}
