//! Ayanamsa: the offset between the tropical and sidereal zodiacs.
//!
//! Each ayanamsa is a fixed direction on the sky: the sidereal zero point. Its tropical
//! longitude at any date is the ayanamsa at that date. Kshana therefore stores one
//! anchor per ayanamsa (the tropical longitude of the zero point at a reference
//! instant, on the true equinox and ecliptic of that instant) and carries that
//! direction to the requested date with the same IAU 2006/2000A precession-nutation
//! it uses for the planets. The result is the ayanamsa on the TRUE equinox of date,
//! which is what must be subtracted from apparent (true-equinox) longitudes.
//!
//! Anchors are the Swiss Ephemeris 2.10.03 values at 2000-01-01T12:00:00 UTC
//! (`get_ayanamsa_ex_ut`, true equinox), recorded in `tests/golden/swisseph.json`;
//! the comparison test shows the carried values agree with Swiss Ephemeris across
//! 1900–2050 to the stated tolerance. True Chitra is a star-based definition (Spica at
//! 180° sidereal) and is carried here as a fixed point without Spica's proper motion,
//! which costs about 0.05″ per year away from 2000.

use kshana_frames::precession::{
    apply, apply_transpose, bias_precession_nutation_matrix, rotate_x,
};
use kshana_frames::{mean_obliquity_iau2006, nutation_iau2006a};

/// Named ayanamsa definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ayanamsa {
    /// Chitrapaksha, as adopted by the Indian Calendar Reform Committee (1955).
    Lahiri,
    /// B. V. Raman.
    Raman,
    /// K. S. Krishnamurti.
    Krishnamurti,
    /// Fagan–Bradley (Western sidereal).
    FaganBradley,
    /// True Chitra: Spica held at exactly 180° sidereal (fixed-point approximation).
    TrueChitra,
}

/// TT Julian centuries since J2000.0 of the anchor instant (2000-01-01T12:00:00 UTC).
const ANCHOR_T: f64 = 0.000_742_870_4 / 36_525.0;

impl Ayanamsa {
    /// Tropical longitude of the sidereal zero point at the anchor instant, degrees,
    /// true equinox of date.
    fn anchor_deg(self) -> f64 {
        match self {
            Ayanamsa::Lahiri => 23.853_222_486_194_408,
            Ayanamsa::Raman => 22.406_921_172_812_087,
            Ayanamsa::Krishnamurti => 23.756_370_172_812_08,
            Ayanamsa::FaganBradley => 24.736_430_126_920_492,
            Ayanamsa::TrueChitra => 23.836_145_182_562_756,
        }
    }
}

/// Unit vector of an ecliptic-of-date longitude (latitude 0) in the GCRS, for TT
/// centuries `t`.
fn zero_point_gcrs(lon_deg: f64, t: f64) -> [f64; 3] {
    let (dpsi, deps) = nutation_iau2006a(t);
    let eps = mean_obliquity_iau2006(t) + deps;
    let (s, c) = lon_deg.to_radians().sin_cos();
    let ecl = [c, s, 0.0];
    let eq = rotate_x(-eps, ecl);
    let m = bias_precession_nutation_matrix(t, dpsi, deps);
    apply_transpose(&m, eq)
}

/// Ayanamsa in degrees at `t` Julian centuries of TT since J2000.0, on the true
/// equinox and ecliptic of date.
pub fn ayanamsa_degrees(which: Ayanamsa, t: f64) -> f64 {
    let v = zero_point_gcrs(which.anchor_deg(), ANCHOR_T);
    let (dpsi, deps) = nutation_iau2006a(t);
    let eps = mean_obliquity_iau2006(t) + deps;
    let m = bias_precession_nutation_matrix(t, dpsi, deps);
    let eq = apply(&m, v);
    let ecl = rotate_x(eps, eq);
    ecl[1].atan2(ecl[0]).to_degrees().rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_reproduces_itself() {
        for a in [
            Ayanamsa::Lahiri,
            Ayanamsa::Raman,
            Ayanamsa::Krishnamurti,
            Ayanamsa::FaganBradley,
            Ayanamsa::TrueChitra,
        ] {
            let v = ayanamsa_degrees(a, ANCHOR_T);
            assert!((v - a.anchor_deg()).abs() < 1e-9, "{a:?}: {v}");
        }
    }

    #[test]
    fn lahiri_in_2026_is_about_24_degrees_13_minutes() {
        let a = ayanamsa_degrees(Ayanamsa::Lahiri, 0.268);
        assert!(a > 24.22 && a < 24.24, "{a}");
    }

    #[test]
    fn rate_is_about_fifty_arcseconds_per_year_on_average() {
        // The true-equinox value carries nutation (±17″), so average over four centuries,
        // where the mean rate is general precession in longitude.
        let a0 = ayanamsa_degrees(Ayanamsa::Lahiri, -2.0);
        let a1 = ayanamsa_degrees(Ayanamsa::Lahiri, 2.0);
        let per_year = (a1 - a0) * 3_600.0 / 400.0;
        assert!((per_year - 50.29).abs() < 0.15, "{per_year}");
    }
}
