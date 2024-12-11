use crate::{
    big_space::{GridCell, GridTransformOwned, ReferenceFrames},
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
        ellipsoid_from_world: DMat4,
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
    pub world_from_local: DMat4,
    local_from_world: DMat4,
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
        translation: DVec3,
        kind: TerrainKind,
    ) -> Self {
        let world_from_local = DMat4::from_scale_rotation_translation(scale, rotation, translation);
        let local_from_world = world_from_local.inverse();

        Self {
            kind,
            translation,
            world_from_local,
            local_from_world,
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
        let ellipsoid_from_world = DMat4::from_rotation_translation(rotation, position).inverse();

        Self::from_scale_rotation_translation(
            DVec3::new(major_axis, minor_axis, major_axis),
            rotation,
            position,
            TerrainKind::ELLIPSOIDAL {
                ellipsoid_from_world,
                major_axis,
                minor_axis,
            },
        )
    }

    pub fn position_local_to_world(&self, local_position: DVec3, height: f64) -> DVec3 {
        let world_position = self.world_from_local.transform_point3(local_position);
        let world_normal = self
            .world_from_local
            .transform_vector3(if self.is_spherical() {
                local_position
            } else {
                DVec3::Y
            })
            .normalize();

        world_position + height * world_normal
    }

    pub fn position_world_to_local(&self, world_position: DVec3) -> DVec3 {
        match self.kind {
            TerrainKind::PLANAR { .. } => {
                DVec3::new(1.0, 0.0, 1.0) * self.local_from_world.transform_point3(world_position)
            }

            TerrainKind::SPHERICAL { .. } => self
                .local_from_world
                .transform_point3(world_position)
                .normalize(),
            TerrainKind::ELLIPSOIDAL {
                ellipsoid_from_world,
                major_axis,
                minor_axis,
            } => {
                let ellipsoid_position = ellipsoid_from_world.transform_point3(world_position);
                let surface_position = project_point_ellipsoid(
                    DVec3::new(major_axis, major_axis, minor_axis),
                    ellipsoid_position,
                );
                self.local_from_world
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
        Transform::from_matrix(self.world_from_local.as_mat4())
    }

    #[cfg(feature = "high_precision")]
    pub(crate) fn grid_transform(
        &self,
        frame: &crate::big_space::ReferenceFrame,
    ) -> crate::big_space::GridTransformOwned {
        let (cell, translation) = frame.translation_to_grid(self.translation);

        crate::big_space::GridTransformOwned {
            transform: Transform::from_matrix(self.world_from_local.as_mat4())
                .with_translation(translation),
            cell,
        }
    }
}

pub fn sync_terrain_position(
    frames: ReferenceFrames,
    mut terrains: Query<(Entity, &mut Transform, &mut GridCell, &TileAtlas)>,
) {
    for (terrain, mut transform, mut cell, tile_atlas) in &mut terrains {
        let frame = frames.parent_frame(terrain).unwrap();
        let GridTransformOwned {
            transform: new_transform,
            cell: new_cell,
        } = tile_atlas.model.grid_transform(frame);

        *transform = new_transform;
        *cell = new_cell;
    }
}
