use bevy::{
    color::palettes::basic,
    math::{DVec2, DVec3},
    prelude::*,
};
use bevy_terrain::{
    math::{Coordinate, SurfaceApproximation},
    prelude::*,
};
use itertools::{iproduct, Itertools};
use rand::{prelude::ThreadRng, rng, Rng};

#[derive(Default)]
struct ViewError {
    position: Vec3,
    max_error: f64,
}

#[derive(Resource, Default)]
struct Errors {
    view_errors: Vec<ViewError>,
    max_error: f64,
}

fn compute_errors() -> Errors {
    let mut rng = rng();

    let shape = TerrainShape::WGS84;

    let view_samples = 1000000;
    let surface_samples = 10;
    let view_lod = 16;
    let threshold = 10000.0;
    let min_tile_lod = (shape.scale() / threshold).log2().ceil() as u32;
    let max_tile_lod = 20;

    // The approximation is as good as the f32 computation (2m max error), at distances below 0.005 * RADIUS (30km) around the camera.
    // With a distance below 0.001 * RADIUS (and an origin lod of 10) the maximum approximation error is around 1 cm.

    let count = view_samples * surface_samples;
    let mut taylor1_max: f64 = 0.0;
    let mut taylor1_avg: f64 = 0.0;
    let mut taylor2_max: f64 = 0.0;
    let mut taylor2_avg: f64 = 0.0;
    let mut f32_max: f64 = 0.0;
    let mut f32_avg: f64 = 0.0;
    let mut cast_max: f64 = 0.0;
    let mut cast_avg: f64 = 0.0;

    let mut view_errors = vec![];

    for _ in 0..view_samples {
        let view_position = random_view_position(&mut rng, shape, threshold);
        let view_coordinate = Coordinate::from_local_position(view_position, shape);

        let view_coordinates = (0..6)
            .map(|face| view_coordinate.project_to_face(face))
            .collect_vec();

        let approximations = view_coordinates
            .iter()
            .map(|&view_coordinate| {
                SurfaceApproximation::compute(view_coordinate, view_position, Vec3::ZERO, shape)
            })
            .collect_vec();

        let mut max_error: f64 = 0.0;

        for _ in 0..surface_samples {
            let sample_position = random_sample_position(&mut rng, shape, threshold, view_position);
            let sample_lod = rng.random_range(min_tile_lod..max_tile_lod);
            let sample_coordinate = sample_coordinate(sample_position, sample_lod, shape);

            let taylor1_error = sample_position.distance(approximate_position(
                view_lod,
                &view_coordinates,
                &approximations,
                false,
                view_position,
                sample_coordinate,
            ));
            let taylor2_error = sample_position.distance(approximate_position(
                view_lod,
                &view_coordinates,
                &approximations,
                true,
                view_position,
                sample_coordinate,
            ));
            let f32_error = sample_position.distance(f32_position(sample_coordinate, shape));
            let cast_error = sample_position.distance(sample_position.as_vec3().as_dvec3());

            taylor1_max = taylor1_max.max(taylor1_error);
            taylor1_avg = taylor1_avg + taylor1_error;
            taylor2_max = taylor2_max.max(taylor2_error);
            taylor2_avg = taylor2_avg + taylor2_error;
            f32_max = f32_max.max(f32_error);
            f32_avg = f32_avg + f32_error;
            cast_max = cast_max.max(cast_error);
            cast_avg = cast_avg + cast_error;
            max_error = max_error.max(taylor2_error);
        }

        view_errors.push(ViewError {
            position: shape.position_local_to_unit(view_position).as_vec3(),
            max_error,
        });
    }

    taylor1_avg = taylor1_avg / count as f64;
    taylor2_avg = taylor2_avg / count as f64;
    f32_avg = f32_avg / count as f64;
    cast_avg = cast_avg / count as f64;

    println!("With a threshold factor of {} and an view LOD of {view_lod}, the error in a sample distance of {:.4} m around the camera looks like this.", threshold / shape.scale(), threshold);
    println!("The world space error introduced by the first order taylor approximation is {:.4} m on average and {:.4} m at the maximum.", taylor1_avg, taylor1_max);
    println!("The world space error introduced by the second order taylor approximation is {:.4} m on average and {:.4} m at the maximum.", taylor2_avg, taylor2_max);
    println!("The world space error introduced by computing the position using f32 is {:.4} m on average and {:.4} m at the maximum.", f32_avg, f32_max);
    println!("The world space error introduced by downcasting from f64 to f32 is {:.4} m on average and {:.4} m at the maximum.", cast_avg, cast_max);

    Errors {
        view_errors,
        max_error: taylor2_max,
    }
}

fn main() {
    let errors = compute_errors();

    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<TransformPlugin>(),
            TerrainPlugin,
            TerrainDebugPlugin,
        ))
        .insert_resource(errors)
        .insert_resource(ClearColor(basic::WHITE.into()))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_big_space(Grid::default(), |root| {
        root.spawn_spatial((
            Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(20).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: basic::BLACK.into(),
                unlit: true,
                ..Default::default()
            })),
        ));

        root.spawn_spatial((
            Transform::from_translation(Vec3::new(1.0, 1.0, 1.0).normalize() * 3.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            DebugCameraController::new(1.0),
        ));
    });
}

fn update(errors: Res<Errors>, mut gizmos: Gizmos) {
    let shape = TerrainShape::Sphere { radius: 1.0 };
    let radius = 0.005;
    let lod = 3;
    let size = 1.0 / (lod as f64).exp2();
    let color = basic::WHITE.darker(0.5);

    for (face, x, y) in iproduct!(0..6, 0..1 << lod, 0..1 << lod) {
        let tile = TileCoordinate::new(face, lod, IVec2::new(x, y));

        for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
            .into_iter()
            .map(|(x, y)| {
                let corner_st = (tile.xy + IVec2::new(x, y)).as_dvec2() * size;
                Coordinate::new(tile.face, corner_st).local_position(shape, radius)
            })
            .tuple_windows()
        {
            gizmos
                .short_arc_3d_between(Vec3::ZERO, start.as_vec3(), end.as_vec3(), color)
                .resolution(20);
        }
    }

    for view_error in &errors.view_errors {
        let rel_error = (view_error.max_error / errors.max_error) as f32;

        let color = basic::BLACK.mix(&basic::RED, rel_error);

        gizmos.sphere(view_error.position, radius * rel_error, color);
    }
}

const C_SQR: f32 = 0.87 * 0.87;

const FACE_MATRICES: [Mat3; 6] = [
    Mat3::from_cols_array(&[-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0]),
    Mat3::from_cols_array(&[0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, -1.0, 0.0]),
    Mat3::from_cols_array(&[0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
    Mat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0]),
    Mat3::from_cols_array(&[0.0, 0.0, -1.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0]),
    Mat3::from_cols_array(&[0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0]),
];

fn f32_position((tile, tile_uv): (TileCoordinate, Vec2), shape: TerrainShape) -> DVec3 {
    let uv = (tile.xy.as_vec2() + tile_uv) / TileCoordinate::count(tile.lod) as f32;

    let xy = (2.0 * uv - 1.0) / (1.0 - 4.0 * C_SQR * (uv - 1.0) * uv).powf(0.5);

    let unit_position = FACE_MATRICES[tile.face as usize] * Vec3::new(1.0, xy.x, xy.y).normalize();

    shape
        .local_from_unit()
        .as_mat4()
        .transform_point3(unit_position)
        .as_dvec3()
}

fn approximate_position(
    view_lod: u32,
    view_coordinates: &[Coordinate],
    approximations: &[SurfaceApproximation],
    second_order: bool,
    view_local_position: DVec3,
    (tile, tile_uv): (TileCoordinate, Vec2),
) -> DVec3 {
    let view_coordinate = view_coordinates[tile.face as usize];

    let uv = view_coordinate.uv * (view_lod as f64).exp2();
    let mut view_xy = uv.as_ivec2();
    let mut view_uv = uv.fract().as_vec2();

    let lod_difference = tile.lod as i32 - view_lod as i32;

    if lod_difference != 0 {
        let scale = (lod_difference as f32).exp2();
        let xy = view_xy;
        let uv = view_uv * scale;

        view_xy = (xy.as_vec2() * scale).as_ivec2() + uv.as_ivec2();
        view_uv = uv.fract()
            + if lod_difference < 0 {
                (xy % (1.0 / scale) as i32).as_vec2() * scale
            } else {
                Vec2::ZERO
            };
    }

    let SurfaceApproximation {
        p,
        p_du,
        p_dv,
        p_duu,
        p_duv,
        p_dvv,
    } = approximations[tile.face as usize].clone();

    let Vec2 { x: u, y: v } =
        ((tile.xy - view_xy).as_vec2() + tile_uv - view_uv) / (tile.lod as f32).exp2();

    let approximate_relative_position = if second_order {
        p + p_du * u + p_dv * v + p_duu * u * u + p_duv * u * v + p_dvv * v * v
    } else {
        p + p_du * u + p_dv * v
    };

    view_local_position + approximate_relative_position.as_dvec3()
}

fn random_sample_position(
    rng: &mut ThreadRng,
    shape: TerrainShape,
    threshold: f64,
    view_local_position: DVec3,
) -> DVec3 {
    shape.position_unit_to_local(
        shape.position_local_to_unit(
            view_local_position
                + (rng.random_range(0.0..1.0)
                    * threshold
                    * DVec3::new(
                        rng.random_range(-1.0..1.0),
                        rng.random_range(-1.0..1.0),
                        rng.random_range(-1.0..1.0),
                    )
                    .normalize()),
        ),
        0.0,
    )
}

fn random_view_position(rng: &mut ThreadRng, shape: TerrainShape, max_height: f64) -> DVec3 {
    Coordinate::new(
        rng.random_range(0..6),
        DVec2::new(rng.random_range(0.0..1.0), rng.random_range(0.0..1.0)),
    )
    .local_position(shape, rng.random_range(0.0..max_height as f32))
}

fn sample_coordinate(
    local_position: DVec3,
    lod: u32,
    shape: TerrainShape,
) -> (TileCoordinate, Vec2) {
    let coordinate = Coordinate::from_local_position(local_position, shape);
    let uv = coordinate.uv * (lod as f64).exp2();
    let tile_xy = uv.as_ivec2();
    let tile_uv = uv.fract().as_vec2();

    (TileCoordinate::new(coordinate.face, lod, tile_xy), tile_uv)
}
