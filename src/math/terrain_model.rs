use crate::{
    big_space::{GridCell, GridTransformOwned, Grids},
    math::ellipsoid::project_point_ellipsoid,
    terrain_data::TileAtlas,
};
use bevy::{
    math::{DMat4, DQuat, DVec3},
    prelude::*,
};

#[derive(Clone)]
pub enum TerrainKind {
    PLANAR {
        side_length: f64,
    },
    SPHERICAL {
        radius: f64,
    },
    ELLIPSOIDAL {
        ellipsoid_from_local: DMat4,
        major_axis: f64,
        minor_axis: f64,
    },
}

// Todo: keep in sync with terrain transform, make this authoritative?
// For this to work, we have to sync the tile_atlas.model with the transform and cell of the terrain
// either we make one authoritative, or we can sync changes both ways
// Todo: make Terrain Model a component?

/// The components of a terrain.
///
/// Does not include loader(s) and a material.
#[derive(Clone)]
pub struct TerrainModel {
    pub(crate) kind: TerrainKind,
    pub local_from_unit: DMat4,
    unit_from_local: DMat4,
    translation: DVec3,
}

impl TerrainModel {
    pub(crate) fn is_spherical(&self) -> bool {
        match self.kind {
            TerrainKind::PLANAR { .. } => false,
            TerrainKind::SPHERICAL { .. } => true,
            TerrainKind::ELLIPSOIDAL { .. } => true,
        }
    }

    fn from_scale_rotation_translation(
        scale: DVec3,
        rotation: DQuat,
        translation: DVec3, // Todo: remove this!
        kind: TerrainKind,
    ) -> Self {
        let local_from_unit = DMat4::from_scale_rotation_translation(scale, rotation, translation);
        let unit_from_local = local_from_unit.inverse();

        Self {
            kind,
            translation,
            local_from_unit,
            unit_from_local,
        }
    }

    pub fn planar(position: DVec3, side_length: f64) -> Self {
        Self::from_scale_rotation_translation(
            DVec3::splat(side_length), // y may not be zero, otherwise local_to_world is NaN
            DQuat::IDENTITY,
            position,
            TerrainKind::PLANAR { side_length },
        )
    }

    pub fn sphere(position: DVec3, radius: f64) -> Self {
        Self::from_scale_rotation_translation(
            DVec3::splat(radius),
            DQuat::IDENTITY,
            position,
            TerrainKind::SPHERICAL { radius },
        )
    }

    pub fn ellipsoid(position: DVec3, major_axis: f64, minor_axis: f64) -> Self {
        let rotation = DQuat::IDENTITY; // ::from_rotation_x(45.0_f64.to_radians());
        let ellipsoid_from_local = DMat4::from_rotation_translation(rotation, position).inverse();

        Self::from_scale_rotation_translation(
            DVec3::new(major_axis, minor_axis, major_axis),
            rotation,
            position,
            TerrainKind::ELLIPSOIDAL {
                ellipsoid_from_local,
                major_axis,
                minor_axis,
            },
        )
    }

    pub fn position_unit_to_local(&self, unit_position: DVec3, height: f64) -> DVec3 {
        let local_position = self.local_from_unit.transform_point3(unit_position);
        let local_normal = self
            .local_from_unit
            .transform_vector3(if self.is_spherical() {
                unit_position
            } else {
                DVec3::Y
            })
            .normalize();

        local_position + height * local_normal
    }

    pub fn position_local_to_unit(&self, local_position: DVec3) -> DVec3 {
        match self.kind {
            TerrainKind::PLANAR { .. } => {
                DVec3::new(1.0, 0.0, 1.0) * self.unit_from_local.transform_point3(local_position)
            }

            TerrainKind::SPHERICAL { .. } => self
                .unit_from_local
                .transform_point3(local_position)
                .normalize(),
            TerrainKind::ELLIPSOIDAL {
                ellipsoid_from_local,
                major_axis,
                minor_axis,
            } => {
                let ellipsoid_position = ellipsoid_from_local.transform_point3(local_position);
                let surface_position = project_point_ellipsoid(
                    DVec3::new(major_axis, major_axis, minor_axis),
                    ellipsoid_position,
                );
                self.unit_from_local
                    .transform_point3(surface_position)
                    .normalize()
            }
        }
    }

    pub fn face_count(&self) -> u32 {
        if self.is_spherical() {
            6
        } else {
            1
        }
    }

    pub fn position(&self) -> DVec3 {
        self.translation
    }

    pub fn scale(&self) -> f64 {
        match self.kind {
            TerrainKind::PLANAR { side_length } => side_length / 2.0,
            TerrainKind::SPHERICAL { radius } => radius,
            TerrainKind::ELLIPSOIDAL {
                major_axis,
                minor_axis,
                ..
            } => (major_axis + minor_axis) / 2.0,
        }
    }

    #[cfg(not(feature = "high_precision"))]
    pub(crate) fn transform(&self) -> Transform {
        Transform::from_matrix(self.local_from_unit.as_mat4())
    }

    #[cfg(feature = "high_precision")]
    pub(crate) fn grid_transform(
        &self,
        frame: &crate::big_space::Grid,
    ) -> crate::big_space::GridTransformOwned {
        let (cell, translation) = frame.translation_to_grid(self.translation);

        crate::big_space::GridTransformOwned {
            transform: Transform::from_matrix(self.local_from_unit.as_mat4())
                .with_translation(translation),
            cell,
        }
    }
}

pub fn sync_terrain_position(
    grids: Grids,
    mut terrains: Query<(Entity, &mut Transform, &mut GridCell, &TileAtlas)>,
) {
    for (terrain, mut transform, mut cell, tile_atlas) in &mut terrains {
        let grid = grids.parent_grid(terrain).unwrap();
        let GridTransformOwned {
            transform: new_transform,
            cell: new_cell,
        } = tile_atlas.model.grid_transform(grid);

        *transform = new_transform;
        *cell = new_cell;
    }
}
