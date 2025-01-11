use crate::math::{Coordinate, TerrainModel, C_SQR, FACE_MATRICES};
use bevy::{
    math::{DMat4, DVec2, DVec3, Vec3},
    render::render_resource::ShaderType,
};

/// Parameters of the view used to compute the position of a location on the sphere's surface relative to the view.
/// This can be calculated directly using f64 operations, or approximated using a Taylor series and f32 operations.
///
/// The idea behind the approximation, is to map from uv coordinates relative to the view, to world positions relative to the view.
/// Therefore, we identify a origin tile with sufficiently high lod (origin LOD), that serves as a reference, to which we can compute our relative coordinate using partly integer math.
#[derive(Copy, Clone, Debug, Default, ShaderType)]
pub struct SurfaceApproximation {
    /// The constant coefficient of the series.
    /// Describes the offset between the location vertically under view and the view position.
    pub c: Vec3,
    /// The linear coefficient of the series with respect to u.
    pub c_du: Vec3,
    /// The linear coefficient of the series with respect to v.
    pub c_dv: Vec3,
    /// The quadratic coefficient of the series with respect to u and u.
    /// This value is pre-multiplied with 0.5.
    pub c_duu: Vec3,
    /// The quadratic coefficient of the series with respect to u and v.
    pub c_duv: Vec3,
    /// The quadratic coefficient of the series with respect to v and v.
    /// This value is pre-multiplied with 0.5.
    pub c_dvv: Vec3,
}

impl SurfaceApproximation {
    /// Computes the view parameters based on the it's world position.
    pub fn compute(
        view_coordinate: Coordinate,
        view_world_position: DVec3,
        model: &TerrainModel,
    ) -> SurfaceApproximation {
        // We want to approximate the position relative to the view using a second order Taylor series.
        // For that, we have to calculate the Taylor coefficients for each cube face separately.
        // As the basis, we use the view coordinate projected to the specific face.
        // Then we calculate the relative position vector and derivatives at the view coordinate.

        // x(u)=(2u-1)/sqrt(1-4cu(u-1))
        // y(v)=(2v-1)/sqrt(1-4cv(v-1))
        // l(u,v)=sqrt(1+x(u)^2+y(v)^2)
        // a(u,v)=1/l(u,v)
        // b(u,v)=x(u)/l(u,v)
        // c(u,v)=y(v)/l(u,v)

        if model.is_spherical() {
            let DVec2 { x: u, y: v } = view_coordinate.uv;
            let face = view_coordinate.face as usize;

            let x_denom = (1.0 - 4.0 * C_SQR * u * (u - 1.0)).sqrt();
            let x = (2.0 * u - 1.0) / x_denom;
            let x_du = 2.0 * (C_SQR + 1.0) / x_denom.powi(3);
            let x_duu = 12.0 * C_SQR * (C_SQR + 1.0) * (2.0 * u - 1.0) / x_denom.powi(5);

            let y_denom = (1.0 - 4.0 * C_SQR * v * (v - 1.0)).sqrt();
            let y = (2.0 * v - 1.0) / y_denom;
            let y_dv = 2.0 * (C_SQR + 1.0) / y_denom.powi(3);
            let y_dvv = 12.0 * C_SQR * (C_SQR + 1.0) * (2.0 * v - 1.0) / y_denom.powi(5);

            let l = (1.0 + x * x + y * y).sqrt();
            let l_du = x * x_du / l;
            let l_dv = y * y_dv / l;
            let l_duu = (x * x_duu * l * l + (y * y + 1.0) * x_du * x_du) / l.powi(3);
            let l_duv = -(x * y * x_du * y_dv) / l.powi(3);
            let l_dvv = (y * y_dvv * l * l + (x * x + 1.0) * y_dv * y_dv) / l.powi(3);

            let a = 1.0;
            let a_du = -l_du;
            let a_dv = -l_dv;
            let a_duu = 2.0 * l_du * l_du - l * l_duu;
            let a_duv = 2.0 * l_du * l_dv - l * l_duv;
            let a_dvv = 2.0 * l_dv * l_dv - l * l_dvv;

            let b = x;
            let b_du = -x * l_du + l * x_du;
            let b_dv = -x * l_dv;
            let b_duu = 2.0 * x * l_du * l_du - l * (2.0 * x_du * l_du + x * l_duu) + x_duu * l * l;
            let b_duv = 2.0 * x * l_du * l_dv - l * (x_du * l_dv + x * l_duv);
            let b_dvv = 2.0 * x * l_dv * l_dv - l * x * l_dvv;

            let c = y;
            let c_du = -y * l_du;
            let c_dv = -y * l_dv + l * y_dv;
            let c_duu = 2.0 * y * l_du * l_du - l * y * l_duu;
            let c_duv = 2.0 * y * l_du * l_dv - l * (y_dv * l_du + y * l_duv);
            let c_dvv = 2.0 * y * l_dv * l_dv - l * (2.0 * y_dv * l_dv + y * l_dvv) + y_dvv * l * l;

            // The model matrix is used to transform the local position and directions into the corresponding world position and directions.
            // p is transformed as a point, thus it takes the model position into account.
            // The other coefficients are transformed as vectors, so they discard the translation.
            let m = model.world_from_local * DMat4::from_mat3(FACE_MATRICES[face]);
            let p = m.transform_point3(DVec3::new(a, b, c) / l);
            let p_du = m.transform_vector3(DVec3::new(a_du, b_du, c_du) / l.powi(2));
            let p_dv = m.transform_vector3(DVec3::new(a_dv, b_dv, c_dv) / l.powi(2));
            let p_duu = m.transform_vector3(DVec3::new(a_duu, b_duu, c_duu) / l.powi(3));
            let p_duv = m.transform_vector3(DVec3::new(a_duv, b_duv, c_duv) / l.powi(3));
            let p_dvv = m.transform_vector3(DVec3::new(a_dvv, b_dvv, c_dvv) / l.powi(3));

            // Todo: cleanup distinction between world space (origin center terrain) and world space (center floating origin)

            SurfaceApproximation {
                c: (p - view_world_position).as_vec3(),
                c_du: p_du.as_vec3(),
                c_dv: p_dv.as_vec3(),
                c_duu: (0.5 * p_duu).as_vec3(),
                c_duv: p_duv.as_vec3(),
                c_dvv: (0.5 * p_dvv).as_vec3(),
            }
        } else {
            SurfaceApproximation {
                c: (view_coordinate.world_position(model, 0.0) - view_world_position).as_vec3(),
                c_du: Vec3::X * model.scale() as f32 * 2.0,
                c_dv: Vec3::Z * model.scale() as f32 * 2.0,
                c_duu: Vec3::ZERO,
                c_duv: Vec3::ZERO,
                c_dvv: Vec3::ZERO,
            }
        }
    }
}
