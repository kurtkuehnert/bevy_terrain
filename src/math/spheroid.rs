use bevy::math::{DVec2, DVec3};
use std::cmp::Ordering;

// Adapted from https://www.geometrictools.com/Documentation/DistancePointEllipseEllipsoid.pdf
// Original licensed under Creative Commons Attribution 4.0 International License
// http://creativecommons.org/licenses/by/4.0/

pub fn project_point_spheroid(major_axis: f64, minor_axis: f64, y: DVec3) -> DVec3 {
    let ellipse = DVec2::new(major_axis, minor_axis);
    let axis = DVec2::new(y.x, y.z);
    let input_position = DVec2::new(axis.length(), y.y);
    let ellipse_position = project_point_ellipse(ellipse, input_position);
    let axis = ellipse_position.x * axis.normalize();

    DVec3::new(axis.x, ellipse_position.y, axis.y)
}

fn project_point_ellipse(ellipse: DVec2, input_position: DVec2) -> DVec2 {
    let sign = input_position.signum();
    let input_position = input_position.abs();

    sign * if input_position.x == 0.0 {
        DVec2::new(0.0, ellipse.y)
    } else if input_position.y == 0.0 {
        let n = ellipse.x * input_position.x;
        let d = ellipse.x * ellipse.x - ellipse.y * ellipse.y;

        if n < d {
            let f = n / d;
            DVec2::new(ellipse.x * f, ellipse.y * (1.0 - f * f).sqrt())
        } else {
            DVec2::new(ellipse.x, 0.0)
        }
    } else {
        let z = input_position / ellipse;
        let g = z.length_squared() - 1.0;

        if g != 0.0 {
            let r = DVec2::new((ellipse.x * ellipse.x) / (ellipse.y * ellipse.y), 1.0);
            input_position * r / (find_root(r, z, g) + r)
        } else {
            input_position
        }
    }
}

fn find_root(r: DVec2, z: DVec2, g: f64) -> f64 {
    let n = r * z;

    let mut s0 = z.y - 1.0;
    let mut s1 = if g < 0.0 { 0.0 } else { n.length() - 1.0 };

    loop {
        let s = (s0 + s1) / 2.0;
        let g = (n / (s + r)).length_squared() - 1.0;

        if s == s0 || s == s1 {
            return s;
        }

        match g.total_cmp(&0.0) {
            Ordering::Less => s1 = s,
            Ordering::Equal => return s,
            Ordering::Greater => s0 = s,
        }
    }
}
