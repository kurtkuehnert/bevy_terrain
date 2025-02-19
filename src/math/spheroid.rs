use bevy::math::{DVec2, DVec3};
use std::cmp::Ordering;

// Adapted from https://www.geometrictools.com/Documentation/DistancePointEllipseEllipsoid.pdf
// Original licensed under Creative Commons Attribution 4.0 International License
// http://creativecommons.org/licenses/by/4.0/

pub fn project_point_spheroid(major_axis: f64, minor_axis: f64, y: DVec3) -> DVec3 {
    let e = DVec2::new(major_axis, minor_axis); // ellipse
    let a = DVec2::new(y.x, y.z); // axis of ellipse
    let y = DVec2::new(a.length(), y.y); // position relative to ellipse
    let x = project_point_ellipse(e, y); // position on ellipse
    let a = x.x * a.normalize(); // axis of spheroid

    DVec3::new(a.x, x.y, a.y) // position on spheroid
}

fn project_point_ellipse(e: DVec2, y: DVec2) -> DVec2 {
    let sign = y.signum();
    let y = y.abs();

    sign * if y.x == 0.0 {
        DVec2::new(0.0, e.y)
    } else if y.y == 0.0 {
        let n = e.x * y.x;
        let d = e.x * e.x - e.y * e.y;

        if n < d {
            let f = n / d;
            DVec2::new(e.x * f, e.y * (1.0 - f * f).sqrt())
        } else {
            DVec2::new(e.x, 0.0)
        }
    } else {
        let z = y / e;
        let g = z.length_squared() - 1.0;

        if g != 0.0 {
            let r = DVec2::new((e.x * e.x) / (e.y * e.y), 1.0);
            y * r / (find_root(r, z, g) + r)
        } else {
            y
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
