use crate::big_space::{
    GridTransform, GridTransformOwned, GridTransformReadOnly, RootReferenceFrame,
};
use crate::prelude::{Terrain, TerrainConfig, TerrainView, TerrainViewComponents};
use bevy::math::{DMat3, DVec2, DVec3, IVec2};
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

/// The square of the parameter c of the algebraic sigmoid function, used to convert between uv and st coordinates.
const C_SQR: f64 = 0.87 * 0.87;

/// One matrix per side, which shuffles the a, b, and c component to their corresponding position.
const SIDE_MATRICES: [DMat3; 6] = [
    DMat3::from_cols_array(&[-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, -1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[0.0, 0.0, -1.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0]),
    DMat3::from_cols_array(&[0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0]),
];

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
pub(crate) struct Coordinate {
    pub(crate) side: u32,
    pub(crate) st: DVec2,
}

impl Coordinate {
    /// Calculates the coordinate for for the local position on the unit cube sphere.
    pub(crate) fn from_local_position(local_position: DVec3) -> Self {
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

    /// Projects the coordinate onto one of the six cube faces.
    /// Thereby it chooses the closest location on this face to the original coordinate.
    fn project_to_side(self, side: u32) -> Self {
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
}

pub(crate) fn tile_count(lod: i32) -> i32 {
    1 << lod
}

/// Parameters of the view used to compute the position of a location on the sphere's surface relative to the view.
/// This can be calculated directly using f64 operations, or approximated using a Taylor series and f32 operations.
///
/// The idea behind the approximation, is to map from st coordinates relative to the view, to world positions relative to the view.
/// Therefore, we identify a origin tile with sufficiently high lod (origin LOD), that serves as a reference, to which we can compute our relative coordinate using partly integer math.
#[derive(Copy, Clone, Debug, Default, ShaderType)]
pub(crate) struct SideParameter {
    /// The tile index of the origin tile projected to this side.
    pub(crate) origin_xy: IVec2,
    /// The offset between the view st coordinate and the origin st coordinate.
    /// This can be used to translate from st coordinates relative to the origin tile to st coordinates relative to the view coordinate in the shader.
    pub(crate) delta_relative_st: Vec2,
    /// The constant coefficient of the series.
    /// Describes the offset between the location vertically under view and the view position.
    pub(crate) c: Vec3,
    /// The linear coefficient of the series with respect to s.
    pub(crate) c_s: Vec3,
    /// The linear coefficient of the series with respect to t.
    pub(crate) c_t: Vec3,
    /// The quadratic coefficient of the series with respect to s and s.
    /// This value is pre-multiplied with 0.5.
    pub(crate) c_ss: Vec3,
    /// The quadratic coefficient of the series with respect to s and t.
    pub(crate) c_st: Vec3,
    /// The quadratic coefficient of the series with respect to t and t.
    /// This value is pre-multiplied with 0.5.
    pub(crate) c_tt: Vec3,
}

#[derive(Clone, Debug, Default, ShaderType)]
pub(crate) struct ModelViewApproximation {
    /// The reference tile, which is used to accurately determine the relative st coordinate in the shader.
    /// The tile under the view (with the origin lod) is the origin for the Taylor series.
    pub(crate) origin_lod: i32,
    pub(crate) view_rotation: Mat4,
    /// The parameters of the six cube sphere faces.
    pub(crate) sides: [SideParameter; 6],
}

impl ModelViewApproximation {
    /// Computes the view parameters based on the it's world position.
    pub(crate) fn compute(
        terrain_transform: GridTransformOwned,
        view_transform: GridTransformOwned,
        radius: f64,
        origin_lod: i32,
        frame: &RootReferenceFrame,
    ) -> ModelViewApproximation {
        let terrain_position = terrain_transform.position_double(&frame);
        let view_position = view_transform.position_double(&frame);
        let view_rotation =
            Mat4::from_rotation_translation(view_transform.transform.rotation, Vec3::ZERO)
                .inverse();

        // dbg!(view_position);
        // dbg!(view_transform.transform.translation);

        let local_position = (view_position - terrain_position) / radius;

        // Coordinate of the location vertically below the view.
        let view_coordinate = Coordinate::from_local_position(local_position);
        // Coordinate of the tile closest to the view coordinate.
        let origin_coordinate = Self::origin_coordinate(view_coordinate, origin_lod);

        // We want to approximate the position relative to the view using a second order Taylor series.
        // For that, we have to calculate the Taylor coefficients for each cube side separately.
        // As the basis, we use the view coordinate projected to the specific side.
        // Then we calculate the relative position vector and derivatives at the view coordinate.

        // u(s)=(2s-1)/sqrt(1-4cs(s-1))
        // v(t)=(2t-1)/sqrt(1-4ct(t-1))
        // l(s,t)=sqrt(1+u(s)^2+v(t)^2)
        // a(s,t)=1/l(s,t)
        // b(s,t)=u(s)/l(s,t)
        // c(s,t)=v(t)/l(s,t)

        let mut sides = [SideParameter::default(); 6];

        for (side, &side_matrix) in SIDE_MATRICES.iter().enumerate() {
            let origin_coordinate = origin_coordinate.project_to_side(side as u32);
            let view_coordinate = view_coordinate.project_to_side(side as u32);
            let origin_xy = (origin_coordinate.st * tile_count(origin_lod) as f64).as_ivec2();
            // The difference between the origin and the view coordinate.
            // This is added to the coordinate relative to the origin tile, in order to get the coordinate relative to the view coordinate.
            // The later serves as the input to this Taylor series.
            let delta_relative_st = (origin_coordinate.st - view_coordinate.st).as_vec2();

            let r = radius;
            let DVec2 { x: s, y: t } = view_coordinate.st;

            let u_denom = (1.0 - 4.0 * C_SQR * s * (s - 1.0)).sqrt();
            let u = (2.0 * s - 1.0) / u_denom;
            let u_ds = 2.0 * (C_SQR + 1.0) / u_denom.powi(3);
            let u_dss = 12.0 * C_SQR * (C_SQR + 1.0) * (2.0 * s - 1.0) / u_denom.powi(5);

            let v_denom = (1.0 - 4.0 * C_SQR * t * (t - 1.0)).sqrt();
            let v = (2.0 * t - 1.0) / v_denom;
            let v_dt = 2.0 * (C_SQR + 1.0) / v_denom.powi(3);
            let v_dtt = 12.0 * C_SQR * (C_SQR + 1.0) * (2.0 * t - 1.0) / v_denom.powi(5);

            let l = (1.0 + u * u + v * v).sqrt();
            let l_ds = u * u_ds / l;
            let l_dt = v * v_dt / l;
            let l_dss = (u * u_dss * l * l + (v * v + 1.0) * u_ds * u_ds) / l.powi(3);
            let l_dst = -(u * v * u_ds * v_dt) / l.powi(3);
            let l_dtt = (v * v_dtt * l * l + (u * u + 1.0) * v_dt * v_dt) / l.powi(3);

            let a = 1.0;
            let a_ds = -l_ds;
            let a_dt = -l_dt;
            let a_dss = 2.0 * l_ds * l_ds - l * l_dss;
            let a_dst = 2.0 * l_ds * l_dt - l * l_dst;
            let a_dtt = 2.0 * l_dt * l_dt - l * l_dtt;

            let b = u;
            let b_ds = -u * l_ds + l * u_ds;
            let b_dt = -u * l_dt;
            let b_dss = 2.0 * u * l_ds * l_ds - l * (2.0 * u_ds * l_ds + u * l_dss) + u_dss * l * l;
            let b_dst = 2.0 * u * l_ds * l_dt - l * (u_ds * l_dt + u * l_dst);
            let b_dtt = 2.0 * u * l_dt * l_dt - l * u * l_dtt;

            let c = v;
            let c_ds = -v * l_ds;
            let c_dt = -v * l_dt + l * v_dt;
            let c_dss = 2.0 * v * l_ds * l_ds - l * v * l_dss;
            let c_dst = 2.0 * v * l_ds * l_dt - l * (v_dt * l_ds + v * l_dst);
            let c_dtt = 2.0 * v * l_dt * l_dt - l * (2.0 * v_dt * l_dt + v * l_dtt) + v_dtt * l * l;

            let p = r * side_matrix * DVec3::new(a, b, c) / l;
            let p_ds = r * side_matrix * DVec3::new(a_ds, b_ds, c_ds) / l.powi(2);
            let p_dt = r * side_matrix * DVec3::new(a_dt, b_dt, c_dt) / l.powi(2);
            let p_dss = r * side_matrix * DVec3::new(a_dss, b_dss, c_dss) / l.powi(3);
            let p_dst = r * side_matrix * DVec3::new(a_dst, b_dst, c_dst) / l.powi(3);
            let p_dtt = r * side_matrix * DVec3::new(a_dtt, b_dtt, c_dtt) / l.powi(3);

            sides[side] = SideParameter {
                origin_xy,

                delta_relative_st,
                c: (p + terrain_position - view_position).as_vec3(),
                c_s: p_ds.as_vec3(),
                c_t: p_dt.as_vec3(),
                c_ss: (p_dss / 2.0).as_vec3(),
                c_st: p_dst.as_vec3(),
                c_tt: (p_dtt / 2.0).as_vec3(),
            };
        }

        ModelViewApproximation {
            origin_lod,
            view_rotation,
            sides,
        }
    }

    /// Computes the view origin tile based on the view's coordinate.
    /// This is a tile with the threshold lod that is closest to the view.
    fn origin_coordinate(coordinate: Coordinate, origin_lod: i32) -> Coordinate {
        let tile_count = tile_count(origin_lod) as f64;
        let st = (coordinate.st * tile_count).round() / tile_count;

        Coordinate { st, ..coordinate }
    }
}

pub(crate) fn generate_model_view_approximation(
    mut model_view_approximations: ResMut<TerrainViewComponents<ModelViewApproximation>>,
    view_query: Query<(Entity, GridTransformReadOnly), With<TerrainView>>,
    terrain_query: Query<(Entity, GridTransformReadOnly, &TerrainConfig), With<Terrain>>,
    frame: Res<RootReferenceFrame>,
) {
    for (terrain, terrain_transform, config) in &terrain_query {
        for (view, view_transform) in &view_query {
            let approximation = ModelViewApproximation::compute(
                terrain_transform.to_owned(),
                view_transform.to_owned(),
                config.scale,
                10,
                &frame,
            );

            model_view_approximations.insert((terrain, view), approximation);
        }
    }
}
