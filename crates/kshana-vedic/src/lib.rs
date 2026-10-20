//! Jyotisha calculations. Every function here is pure arithmetic on longitudes and
//! instants; the sky itself comes from `kshana-ephem`.

#![forbid(unsafe_code)]

pub mod ashtakavarga;
pub mod ayanamsa;
pub mod dasha;
pub mod dashas;
pub mod jaimini;
pub mod kp;
pub mod naming;
pub mod panchanga;
pub mod varga;

pub use ashtakavarga::{
    bhinna, ekadhipatya_shodhana, pinda, reduce, sarva, trikona_shodhana, Pinda,
    Positions as AshtakavargaPositions, Reduction, ROW_TOTALS,
};
pub use ayanamsa::{ayanamsa_degrees, Ayanamsa};
pub use dasha::{subdivide, vimshottari, vimshottari_levels, DashaLord, DashaPeriod};
pub use jaimini::{
    arudha, arudha_padas, chara_karakas, chara_karakas_8, sign_aspects, signs_aspect, Karaka,
    KarakaScheme, KARAKA_MEANINGS, KARAKA_NAMES, KARAKA_NAMES_8,
};
pub use kp::{
    dms_in_sign, horary_longitude, kp_lords, lord_chain, sign_lord, sub_bounds, sub_of, sub_table,
    ChainLevel, KpLords, Sub,
};
pub use naming::{naming_for, syllable, SYLLABLES};
pub use panchanga::{
    karana, nakshatra, tithi, yoga, Karana, Nakshatra, Tithi, Yoga, NAKSHATRA_SPAN,
};
pub use varga::{nadiamsa_ordinal, varga, varga_sign, SUPPORTED_VARGAS};

/// Normalise a longitude into `[0, 360)`.
pub fn norm360(d: f64) -> f64 {
    d.rem_euclid(360.0)
}

/// Sidereal longitude from a tropical one and an ayanamsa, both degrees.
pub fn sidereal(tropical: f64, ayanamsa: f64) -> f64 {
    norm360(tropical - ayanamsa)
}
