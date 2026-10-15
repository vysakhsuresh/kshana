//! Ayanamsa: the offset between the tropical and sidereal zodiacs.
//!
//! Every ayanamsa is defined by a value at an epoch plus the general precession in
//! longitude. This scaffold carries the epoch values and the IAU 2006 precession rate;
//! milestone 3 verifies each against its reference source (Indian Astronomical
//! Ephemeris for Lahiri, Swiss Ephemeris and the original authors for the others)
//! to 0.01″ and records the comparison in `docs/PRECISION.md`.

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
    /// True Chitra: Spica held at exactly 180° sidereal (needs Spica's position; the
    /// value here is the mean-Chitra approximation until the star catalogue lands).
    TrueChitra,
}

impl Ayanamsa {
    /// Value at J2000.0 in degrees.
    fn at_j2000(self) -> f64 {
        match self {
            Ayanamsa::Lahiri => 23.853_080,
            Ayanamsa::Raman => 22.386_875,
            Ayanamsa::Krishnamurti => 23.747_369,
            Ayanamsa::FaganBradley => 24.736_944,
            Ayanamsa::TrueChitra => 23.853_080,
        }
    }
}

/// General precession in longitude since J2000.0, degrees, IAU 2006 (P03), for `t`
/// Julian centuries of TT.
fn precession_longitude_deg(t: f64) -> f64 {
    (5_028.796_195 * t + 1.105_434_8 * t * t + 0.000_079_64 * t * t * t) / 3_600.0
}

/// Ayanamsa in degrees at `t` Julian centuries of TT since J2000.0.
pub fn ayanamsa_degrees(which: Ayanamsa, t: f64) -> f64 {
    which.at_j2000() + precession_longitude_deg(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lahiri_in_2026_is_about_24_degrees_13_minutes() {
        let t = 0.268; // ~ 2026.8
        let a = ayanamsa_degrees(Ayanamsa::Lahiri, t);
        assert!(a > 24.22 && a < 24.24, "{a}");
    }

    #[test]
    fn rate_is_about_fifty_arcseconds_per_year() {
        let a0 = ayanamsa_degrees(Ayanamsa::Lahiri, 0.0);
        let a1 = ayanamsa_degrees(Ayanamsa::Lahiri, 0.01);
        let per_year_arcsec = (a1 - a0) * 3_600.0;
        assert!((per_year_arcsec - 50.29).abs() < 0.05, "{per_year_arcsec}");
    }
}
