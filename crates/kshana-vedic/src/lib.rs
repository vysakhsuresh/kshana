//! Jyotisha calculations. Every function here is pure arithmetic on longitudes and
//! instants; the sky itself comes from `kshana-ephem`.

#![forbid(unsafe_code)]

pub mod ayanamsa;
pub mod dasha;
pub mod panchanga;

pub use ayanamsa::{ayanamsa_degrees, Ayanamsa};
pub use dasha::{vimshottari, DashaLord, DashaPeriod};
pub use panchanga::{
    karana, nakshatra, tithi, yoga, Karana, Nakshatra, Tithi, Yoga, NAKSHATRA_SPAN,
};

/// Normalise a longitude into `[0, 360)`.
pub fn norm360(d: f64) -> f64 {
    d.rem_euclid(360.0)
}

/// Sidereal longitude from a tropical one and an ayanamsa, both degrees.
pub fn sidereal(tropical: f64, ayanamsa: f64) -> f64 {
    norm360(tropical - ayanamsa)
}
