//! The Kerala Prasna Marga suite and the Jaimini extras a Jyotishi expects next to a
//! Parashari chart.
//!
//! * [`sphuta`]: the Prasna Marga sensitive points: Trisphuta, Chatussphuta and
//!   Panchasphuta, the Prana, Deha and Mrityu sphutas, the Sookshma Trisphuta, the
//!   Beeja and Kshetra sphutas, the Yogi point with its Yogi, Sahayogi and Avayogi, and
//!   the Bhrigu Bindu, with the text's readings carried as data.
//! * [`ayur`]: the three classical longevity arithmetics of Brihat Parashara Hora
//!   Shastra, Pindayu, Nisargayu and Amsayu, with the haranas, the bharanas, the lagna's
//!   share and the rule that picks the method. The result says, in its own words, that
//!   this is a classical exercise and not a prediction of death.
//! * [`ashtamangala`]: the Ashtamangala number of the Kerala prasna and the Arudha,
//!   Veethi and Chatra rasis with the lagna, Chandra and Arudha sphutas.
//! * [`jaimini`]: rasi drishti, argala and virodhargala, the karakamsa, and the Pada
//!   lagna and Upapada.
//! * [`sandhi`]: Vimshottari junction windows as exact instants, and the chidra lords.
//! * [`vimshopaka`]: Vimshopaka bala in the Shadvarga, Saptavarga, Dashavarga and
//!   Shodashavarga schemes.
//!
//! Every longitude is sidereal in the chart's own ayanamsa, every instant is exact, and
//! nothing here casts a sky of its own: the chart comes from `kshana_chart`, Gulika from
//! `kshana_day`, strengths from `kshana_bala`. Where a rule is read differently by
//! different authors the function says which reading it uses and names the others; such
//! functions carry the word *convention* in their documentation.

#![forbid(unsafe_code)]

pub mod ashtamangala;
pub mod ayur;
pub mod jaimini;
pub mod sandhi;
pub mod sphuta;
pub mod vimshopaka;

pub use ashtamangala::{
    ashtamangala, chatra_rasi, digit_planet, prasna_sphutas, veethi_rasi, Ashtamangala,
    PrasnaSphutas, DIGIT_SIGNALS,
};
pub use ayur::{
    ayurdaya, ayurdaya_all, ayurdaya_by, ayurdaya_with_shadbala, exaltation_arc, nisargayu_years,
    pindayu_years, proportioned_years, select_methods, AyurInput, AyurMethod, Ayurdaya,
    Contribution, Reduction, Strengths, LONGEVITY_CAVEAT, PARAMAYU,
};
pub use jaimini::{
    argala, argala_on_sign, karakamsa, pada_lagna, pada_of, rasi_aspects, rasi_drishti, upapada,
    Argala, ArgalaPair, Karakamsa, Pada, KARAKAMSA_RESULTS,
};
pub use sandhi::{
    chidra_lords, khara_drekkana_lord, navamsa_64_lord, sandhi_windows, sandhi_windows_with,
    Sandhi, SandhiRule,
};
pub use sphuta::{
    beeja_sphuta, bhrigu_bindu, chatussphuta, dehasphuta, gulika_longitude, gulika_longitude_with,
    kshetra_sphuta, mrityusphuta, panchasphuta, pranasphuta, sookshma_trisphuta, sphuta_report,
    sphutas, trisphuta, trisphuta_words, yogi, GulikaConvention, GulikaPosition, Sphuta,
    SphutaReport, Yogi, TRISPHUTA_TRINE_EVENTS,
};
pub use vimshopaka::{
    grade, relation_points, varga_points, varga_sign, vimshopaka, vimshopaka_of, Vimshopaka,
    DASHAVARGA, SAPTAVARGA, SHADVARGA, SHODASHAVARGA,
};

use kshana_bala::BalaError;
use kshana_ephem::SpkError;
use kshana_events::EventError;
use kshana_vedic::DashaLord;

/// Names of the twelve signs, 0 = Aries.
pub const SIGN_NAMES: [&str; 12] = [
    "Aries",
    "Taurus",
    "Gemini",
    "Cancer",
    "Leo",
    "Virgo",
    "Libra",
    "Scorpio",
    "Sagittarius",
    "Capricorn",
    "Aquarius",
    "Pisces",
];

/// Why a computation that needs the sky could not be done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MargaError {
    /// Ephemeris failure.
    Spk(SpkError),
    /// Sunrise search failure.
    Events(EventError),
    /// No sunrise and sunset frame the instant (polar day or night).
    NoSolarDay,
    /// Shadbala failure.
    Bala(BalaError),
    /// The chart lacks a body.
    MissingBody(&'static str),
}

impl From<SpkError> for MargaError {
    fn from(e: SpkError) -> Self {
        MargaError::Spk(e)
    }
}
impl From<EventError> for MargaError {
    fn from(e: EventError) -> Self {
        MargaError::Events(e)
    }
}
impl From<BalaError> for MargaError {
    fn from(e: BalaError) -> Self {
        MargaError::Bala(e)
    }
}

/// Normalise a longitude into `[0, 360)`.
pub fn norm360(d: f64) -> f64 {
    d.rem_euclid(360.0)
}

/// Sign index 0–11 of a longitude.
pub fn sign_of(longitude: f64) -> u8 {
    ((norm360(longitude) / 30.0).floor() as usize % 12) as u8
}

/// English name of a graha.
pub fn graha_name(g: DashaLord) -> &'static str {
    kshana_bala::graha_name(g)
}

/// Sidereal longitude of a named body in a chart.
pub(crate) fn body_sidereal(
    chart: &kshana_chart::Chart,
    name: &'static str,
) -> Result<f64, MargaError> {
    chart
        .bodies
        .iter()
        .find(|b| b.name == name)
        .map(|b| b.sidereal)
        .ok_or(MargaError::MissingBody(name))
}

#[cfg(test)]
pub(crate) mod fixture {
    use kshana_chart::{compute, Chart, ChartInput};
    use kshana_ephem::Spk;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::{Instant, Utc};
    use kshana_vedic::Ayanamsa;

    /// Kochi, the fixture place used across the workspace.
    pub const KOCHI: Observer = Observer {
        latitude_deg: 9.9312,
        longitude_deg: 76.2673,
        height_m: 0.0,
    };

    pub fn kernel() -> Option<Spk> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        Spk::open(&p).ok()
    }

    pub fn utc(y: i64, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> Instant {
        Utc::new(y, mo, d, h, mi, s, 0)
            .unwrap()
            .to_instant()
            .unwrap()
    }

    /// The Kochi birth: 1990-10-29 12:06 UTC, Placidus, Lahiri.
    pub fn kochi_birth() -> Instant {
        utc(1990, 10, 29, 12, 6, 0)
    }

    pub fn kochi_chart(spk: &Spk) -> Chart {
        compute(
            spk,
            None,
            &ChartInput {
                instant: kochi_birth(),
                observer: KOCHI,
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_helpers() {
        assert_eq!(sign_of(0.0), 0);
        assert_eq!(sign_of(359.9), 11);
        assert_eq!(sign_of(-1.0), 11);
        assert_eq!(norm360(370.0), 10.0);
        assert_eq!(SIGN_NAMES[sign_of(125.0) as usize], "Leo");
        assert_eq!(graha_name(DashaLord::Ketu), "Ketu");
    }
}
