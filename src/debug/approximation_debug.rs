use crate::{
    math::{Coordinate, SurfaceApproximation},
    prelude::TileTree,
    terrain_view::TerrainViewComponents,
};
use bevy::{color::palettes::basic, math::DVec2, prelude::*};
use itertools::Itertools;

const DEBUG_SCALE: f32 = 1.0 / (1 << 5) as f32;

pub(crate) fn debug_surface_approximation(
    mut enable: Local<bool>,
    mut gizmos: Gizmos,
    tile_trees: Res<TerrainViewComponents<TileTree>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyD) {
        *enable = !*enable;
    }

    if !*enable {
        return;
    }

    for tile_tree in tile_trees.values() {
        let shape = tile_tree.shape;

        for face in 0..shape.face_count() {
            let SurfaceApproximation {
                p,
                p_du,
                p_dv,
                p_duu,
                p_duv,
                p_dvv,
            } = tile_tree.surface_approximation[face as usize].clone();

            let height = shape.scale() as f32 * 0.02;
            let normal = p_dv.cross(p_du).normalize();
            let position = p + height * normal;

            gizmos.sphere(
                position,
                0.01 * DEBUG_SCALE * shape.scale() as f32,
                basic::OLIVE,
            );
            gizmos.arrow(position, position + p_du * DEBUG_SCALE, basic::YELLOW);
            gizmos.arrow(position, position + p_dv * DEBUG_SCALE, basic::GREEN);
            gizmos.arrow(position, position + p_duu * DEBUG_SCALE, basic::RED);
            gizmos.arrow(position, position + p_duv * DEBUG_SCALE, basic::BLUE);
            gizmos.arrow(position, position + p_dvv * DEBUG_SCALE, basic::FUCHSIA);

            let view_coordinate = &tile_tree.view_coordinates[face as usize];

            for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
                .into_iter()
                .map(|(x, y)| {
                    let corner_uv = (view_coordinate.uv
                        + DVec2::new(2.0 * x as f64 - 1.0, 2.0 * y as f64 - 1.0)
                            * DEBUG_SCALE as f64)
                        .clamp(DVec2::splat(0.0), DVec2::splat(1.0));
                    Coordinate::new(face, corner_uv).local_position(shape, height)
                })
                .tuple_windows()
            {
                gizmos.short_arc_3d_between(
                    -tile_tree.view_local_position.as_vec3(),
                    (start - tile_tree.view_local_position).as_vec3(),
                    (end - tile_tree.view_local_position).as_vec3(),
                    Color::WHITE,
                );
            }
        }
    }
}
