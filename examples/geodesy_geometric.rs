// examples/01-geometric_geodesy.rs

// Using Rust Geodesy for simple geodetic computations.
// Run with:
// cargo run --example 01-geometric_geodesy

// Geodetic naming conventions conflict somewhat with
// Rust's (otherwise sensible) conventions, so we turn
// snake case warnings of for this entire file.
#![allow(non_snake_case)]

use geodesy::prelude::*;

fn main() -> Result<(), Error> {
    // In example 00, we saw that the `Context` data structure is the
    // coordinating element for all things related to transformations
    // in Rust Geodesy. For generic geometric geodesy the same can be
    // said about the `Ellipsoid`. So to do anything, we must first
    // instantiate an `Ellipsoid`. We can do that in two ways - either
    // by asking, by name, for one of the built in ellipsoids, or by
    // providing our own ellipsoid parameters:

    // The GRS 1980 ellipsoid is built in, so we use the ::named function.
    let WGS84 = Ellipsoid::named("WGS84")?;

    // The Maupertuis 1738 ellipsoid is not built in, so we provide `a`,
    // the semimajor axis, and `f`, the flattening to the `new()`
    // constructor.
    let Mau38 = Ellipsoid::new(6_397_300., 1.0 / 191.);

    // Now, let's compute som ancillary ellipsoidal parameters:
    let E = WGS84.linear_eccentricity();
    let b = WGS84.semiminor_axis();
    let c = WGS84.polar_radius_of_curvature();
    let n = WGS84.third_flattening();
    let es = WGS84.eccentricity_squared();
    println!("WGS84 - Ellipsoid parameters");
    println!("    E   =  {E:.4}");
    println!("    b   =  {b:.4}");
    println!("    c   =  {c:.4}");
    println!("    n   =  {n}");
    println!("    es  =  {es}");
    println!();

    // A geodesic is the shortest line between two points on the
    // surface of the ellipsoid. Let's compute the distance and
    // azimuth between the approximate locations of the airports
    // of Copenhagen (CPH) and Paris (CDG).
    let CPH = Coor2D::geo(55., 12.);
    let CDG = Coor2D::geo(49., 2.);

    // By historical convention the "from A to B" situation is considered
    // the inverse sense of the geodesic problem - hence `geodesic_inv`:
    let d = WGS84.geodesic_inv(&CPH, &CDG);
    let dd = d.to_degrees();
    // Note the '.to_degrees()' above: This Coord method attacks
    // the first two elements of the coordinate only. The output from the
    // geodesic routines is organized to fit this pattern.

    println!("WGS84 - Copenhagen->Paris, inv algorithm");
    println!("    Distance:                {:.3} km", dd[2] / 1000.);
    println!("    Azimuth at departure:    {:.1} deg", dd[0]);
    println!("    Azimuth at destination:  {:.1} deg", dd[1]);
    println!();

    // Now we have the azimuth from CPH to CDG - so let's take the same trip
    // again, this time using the "forward" version:
    let b = WGS84.geodesic_fwd(&CPH, d[0], d[2]).to_degrees();
    // In this case, output is [longitude, latitude, 0, 0]
    println!("WGS84 - Copenhagen->Paris, fwd algorithm");
    println!("    Location:   {} {}", b[0], b[1]);
    println!();

    // We assert to hit the spot within a nanometer
    assert!((b[0] - 2.).abs() < 1e-9);
    assert!((b[1] - 49.).abs() < 1e-9);

    // Let's try going back using the azimuth at the destination.
    // We need to swap its direction to get us back to Copenhagen.
    let az_back = (dd[1] + 180.0).to_radians();
    let b = WGS84.geodesic_fwd(&CDG, az_back, d[2]).to_degrees();
    println!("WGS84 - Paris->Copenhagen, fwd algorithm, with swapped azimuth");
    println!("    Location:   {} {}", b[0], b[1]);
    println!();

    let d = WGS84.geodesic_inv(&CDG, &CPH);
    let dd = d.to_degrees();
    // Note the '.to_degrees()' above: This Coord method attacks
    // the first two elements of the coordinate only. The output from the
    // geodesic routines is organized to fit this pattern.
    println!("WGS84 - Paris->Copenhagen");
    println!("    Distance:                {:.3} km", dd[2] / 1000.);
    println!("    Azimuth at departure:    {:.1} deg", dd[0]);
    println!("    Azimuth at destination:  {:.1} deg", dd[1]);
    println!();

    // But how would it be, if we were not handling a Boeing 737 on the
    // WGS84 ellipsoid in 2021, but a Montgolfière on the Maupertuis
    // ellipsoid in 1783?
    let dd = Mau38.geodesic_inv(&CPH, &CDG).to_degrees();

    println!("Mau38 - Copenhagen->Paris, inv algorithm");
    println!("    Distance:                {:.3} km", dd[2] / 1000.);
    println!("    Azimuth at departure:    {:.1} deg", dd[0]);
    println!("    Azimuth at destination:  {:.1} deg", dd[1]);
    println!();

    // So the Montgolfier brothers would have thought they had flown
    // approximately 3 km longer than the modern day airline pilot.
    Ok(())
}
