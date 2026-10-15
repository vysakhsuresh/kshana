//! Reference frames and Earth orientation.
//!
//! Implemented to IAU 2006/2000 resolutions where the formula is short enough to be
//! audited by eye. Anything with a large coefficient table (full IAU 2000A nutation,
//! P03 precession matrices) ships as data in milestone 2 and is marked below.

#![forbid(unsafe_code)]

use kshana_time::TwoPartJd;
use std::f64::consts::{PI, TAU};

/// Arcseconds to radians.
pub const ARCSEC: f64 = PI / (180.0 * 3_600.0);

/// Earth Rotation Angle (IAU 2000) in radians for a UT1 two-part Julian Date.
///
/// θ = 2π (F + 0.779 057 273 264 0 + 0.002 737 811 911 354 48 · Tu), with Tu the UT1
/// days since J2000.0 and F its fractional part. The two-part form keeps F exact.
pub fn earth_rotation_angle(ut1: TwoPartJd) -> f64 {
    let t = (ut1.jd1 - 2_451_545.0) + ut1.jd2;
    let f = ut1.jd1.rem_euclid(1.0) + ut1.jd2.rem_euclid(1.0);
    let theta = TAU * (f + 0.779_057_273_264_0 + 0.002_737_811_911_354_48 * t);
    theta.rem_euclid(TAU)
}

/// Mean obliquity of the ecliptic (IAU 2006) in radians, `t` in TT centuries since J2000.
pub fn mean_obliquity_iau2006(t: f64) -> f64 {
    (84_381.406
        + t * (-46.836_769
            + t * (-0.000_183_1
                + t * (0.002_003_40 + t * (-0.000_000_576 + t * -0.000_000_043_4)))))
        * ARCSEC
}

/// Greenwich Mean Sidereal Time (IAU 2006) in radians, from ERA and TT centuries.
pub fn gmst_iau2006(era: f64, t: f64) -> f64 {
    let poly = 0.014_506
        + t * (4_612.156_534
            + t * (1.391_581_7
                + t * (-0.000_000_44 + t * (-0.000_029_956 + t * -0.000_000_036_8))));
    (era + poly * ARCSEC).rem_euclid(TAU)
}

/// Nutation in longitude and obliquity, radians, TRUNCATED to the four largest terms.
///
/// Accuracy about 1″. This exists so the scaffold produces a true-equinox sidereal
/// time with the right shape; milestone 2 replaces it with the full IAU 2000A series
/// (1 365 terms, ~0.1 mas), loaded from a data table and tested against SOFA.
pub fn nutation_truncated(t: f64) -> (f64, f64) {
    // Fundamental arguments (degrees), Delaunay variables, low-order.
    let omega = (125.044_52 - 1_934.136_261 * t).to_radians();
    let l_sun = (280.466_5 + 36_000.769_8 * t).to_radians();
    let l_moon = (218.316_5 + 481_267.881_3 * t).to_radians();
    let dpsi = -17.20 * omega.sin() - 1.32 * (2.0 * l_sun).sin() - 0.23 * (2.0 * l_moon).sin()
        + 0.21 * (2.0 * omega).sin();
    let deps = 9.20 * omega.cos() + 0.57 * (2.0 * l_sun).cos() + 0.10 * (2.0 * l_moon).cos()
        - 0.09 * (2.0 * omega).cos();
    (dpsi * ARCSEC, deps * ARCSEC)
}

/// Greenwich Apparent Sidereal Time: GMST plus the equation of the equinoxes.
pub fn gast(era: f64, t: f64) -> f64 {
    let (dpsi, deps) = nutation_truncated(t);
    let eps = mean_obliquity_iau2006(t) + deps;
    (gmst_iau2006(era, t) + dpsi * eps.cos()).rem_euclid(TAU)
}

/// Local sidereal time from a Greenwich value and an east longitude (radians).
pub fn local_sidereal_time(greenwich: f64, east_longitude: f64) -> f64 {
    (greenwich + east_longitude).rem_euclid(TAU)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deg(r: f64) -> f64 {
        r.to_degrees()
    }

    #[test]
    fn era_at_j2000() {
        let era = earth_rotation_angle(TwoPartJd {
            jd1: 2_451_545.0,
            jd2: 0.0,
        });
        assert!((deg(era) - 280.460_618_37).abs() < 1e-8, "{}", deg(era));
    }

    #[test]
    fn era_two_part_matches_one_part_for_small_values() {
        let a = earth_rotation_angle(TwoPartJd {
            jd1: 2_451_545.0,
            jd2: 0.25,
        });
        let b = earth_rotation_angle(TwoPartJd {
            jd1: 2_451_545.25,
            jd2: 0.0,
        });
        assert!((a - b).abs() < 1e-12);
    }

    #[test]
    fn obliquity_at_j2000() {
        let e = mean_obliquity_iau2006(0.0);
        assert!((deg(e) - 23.439_279_444).abs() < 1e-9);
    }

    #[test]
    fn gmst_at_j2000() {
        let era = earth_rotation_angle(TwoPartJd {
            jd1: 2_451_545.0,
            jd2: 0.0,
        });
        let g = gmst_iau2006(era, 0.0);
        // 280.46061837 + 0.014506″
        assert!((deg(g) - 280.460_622_4).abs() < 1e-6, "{}", deg(g));
    }

    #[test]
    fn nutation_has_the_right_magnitude() {
        let (dpsi, deps) = nutation_truncated(0.0);
        assert!(dpsi.abs() < 20.0 * ARCSEC);
        assert!(deps.abs() < 10.0 * ARCSEC);
    }
}
