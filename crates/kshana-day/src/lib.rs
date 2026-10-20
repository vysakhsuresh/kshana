//! The day's clocks: everything Jyotisha divides a day into, with every boundary an
//! exact instant derived from the solved sunrise or from a solved root.
//!
//! * [`clock`]: ghati and vighati readouts under the named conventions, the thirty
//!   muhurtas with their classical names, Abhijit, Brahma muhurta and the sixteen
//!   choghadiya of day and night.
//! * [`upagraha`]: Gulika, Mandi, Kala, Mrityu, Ardhaprahara and Yamaghantaka as kalas
//!   (eighths of the day or night, with the start, middle and end conventions) and as
//!   longitudes (the sidereal ascendant at the chosen instant of the kala), the popular
//!   Yamaganda and Gulika kala tables, and the five arithmetic upagrahas from the Sun.
//! * [`lagna`]: the BPHS sub-hour lagnas (Bhava, Hora, Ghati, Vighati, Pranapada and
//!   Sree) as longitude functions of time, with the exact instants at which they change
//!   sign.
//! * [`windows`]: Gandanta, Panchaka, Bhadra (Vishti karana), the named inauspicious
//!   yogas, tara bala and chandra bala as exact windows of the transiting Moon.
//! * [`pakshi`]: Pancha Pakshi, the five birds and their five activities over the
//!   solar day.
//!
//! Precision: a sunrise-derived edge is exact arithmetic on a sunrise that
//! `kshana_events::solar_day` solves to a millisecond, but the physical sunrise is
//! limited by refraction and the horizon to tens of seconds, so these edges should be
//! shown with that bracket. A transit edge (the Moon crossing a sidereal boundary) is
//! a root solved to a microsecond and carries its own bracket width.

#![forbid(unsafe_code)]

pub mod clock;
pub mod lagna;
pub mod pakshi;
pub mod upagraha;
pub mod windows;

pub use clock::{
    abhijit, brahma_muhurta, brahma_muhurta_classical, choghadiya, choghadiya_at, choghadiya_table,
    first_choghadiya_lord, ghati_length_ns, ghati_vighati, muhurta_at, muhurtas, Choghadiya,
    ChoghadiyaWindow, GhatiConvention, MuhurtaName, MuhurtaQuality, MuhurtaWindow, GHATI_NS,
    MUHURTAS, VIGHATI_NS, VIPALA_NS,
};
pub use lagna::{
    ghati_family_lagna, lagna_sign_changes, pranapada, pranapada_at, pranapada_auspicious,
    pranapada_offset, special_lagna_at, sree_lagna, sree_lagna_at, sun_sidereal_at, LagnaChange,
    LagnaContext, SpecialLagna, SunAnchor,
};
pub use pakshi::{
    activity_cycle, birth_bird, birth_bird_at, pancha_pakshi_day, pancha_pakshi_sub_windows,
    pancha_pakshi_windows, ruling_bird, yama_activity, Activity, Bird, PakshiSubWindow,
    PakshiWindow, SubPeriodDurations, ACTIVITY_TABLE, SUB_PERIOD_DAY, SUB_PERIOD_NIGHT,
};
pub use upagraha::{
    gulika_kala, mandi_instant_kerala, part_lord_name, part_lords, parts, solar_upagrahas,
    upagraha_instant, upagraha_kala, upagraha_longitude, upagraha_longitudes, yamaganda_kala,
    KalaPoint, Upagraha, UpagrahaPosition, GULIKA_SLOTS, YAMAGANDA_SLOTS,
};
pub use windows::{
    bhadra_windows, chandra_bala_windows, gandanta_windows, panchaka_windows, special_yoga_windows,
    tara_bala_windows, yoga_windows, LabelledWindow, CHANDRA_LABELS, INAUSPICIOUS_YOGAS,
    TARA_NAMES,
};

use kshana_chart::ut1_for;
use kshana_ephem::{Body, Spk, SpkError};
use kshana_events::{solar_day, EventError, SolarDay};
use kshana_frames::{
    earth_rotation_angle, gast, local_sidereal_time, mean_obliquity_iau2006, nutation_iau2006a,
};
use kshana_houses::{angles, HouseError};
use kshana_sky::{apparent, Observer};
use kshana_time::{EopTable, Instant, Utc, NS_PER_SEC};
use kshana_vedic::{ayanamsa_degrees, sidereal, Ayanamsa};

/// Why a clock could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DayError {
    /// Ephemeris failure.
    Spk(SpkError),
    /// Sunrise or root search failure.
    Events(EventError),
    /// The ascendant is undefined at the place.
    Houses(HouseError),
    /// No sunrise brackets the instant (polar day or night, or outside the kernel).
    NoSolarDay,
}

impl From<SpkError> for DayError {
    fn from(e: SpkError) -> Self {
        DayError::Spk(e)
    }
}
impl From<EventError> for DayError {
    fn from(e: EventError) -> Self {
        DayError::Events(e)
    }
}
impl From<HouseError> for DayError {
    fn from(e: HouseError) -> Self {
        DayError::Houses(e)
    }
}

/// Weekday lords in weekday order, index 0 = Sunday.
pub const WEEKDAY_LORDS: [&str; 7] = [
    "Sun", "Moon", "Mars", "Mercury", "Jupiter", "Venus", "Saturn",
];

/// Weekday (0 = Sunday) of the solar day: the local civil date at its sunrise, the
/// local offset taken from the observer's longitude (the same rule as
/// `kshana_events::vara_name`).
pub fn weekday(day: &SolarDay, obs: &Observer) -> u32 {
    let local = day
        .sunrise
        .add_ns((obs.longitude_deg / 15.0 * 3_600.0 * NS_PER_SEC as f64) as i128);
    ((Utc::from_instant(local).utc_day() + 4).rem_euclid(7)) as u32
}

/// Sidereal longitude (degrees) of the rising point at `t` for the observer: the
/// Ascendant from local apparent sidereal time and the true obliquity, less the
/// ayanamsa of date. The same chain as `kshana_chart::compute` uses for cusp 1.
pub fn ascendant_sidereal(
    eop: Option<&EopTable>,
    obs: &Observer,
    t: Instant,
    ayanamsa: Ayanamsa,
) -> f64 {
    let tc = t.tt_centuries_since_j2000();
    let (ut1, _) = ut1_for(t, eop);
    let era = earth_rotation_angle(ut1);
    let lst = local_sidereal_time(gast(era, tc), obs.longitude_deg.to_radians());
    let (_, deps) = nutation_iau2006a(tc);
    let eps = mean_obliquity_iau2006(tc) + deps;
    let asc = angles(lst, eps, obs.latitude_deg.to_radians()).asc;
    sidereal(asc, ayanamsa_degrees(ayanamsa, tc))
}

/// Apparent sidereal longitude (degrees) of `body` at `t`.
pub fn sidereal_longitude(
    spk: &Spk,
    body: Body,
    t: Instant,
    ayanamsa: Ayanamsa,
) -> Result<f64, SpkError> {
    let a = apparent(spk, body, t)?;
    Ok(sidereal(
        a.longitude,
        ayanamsa_degrees(ayanamsa, t.tt_centuries_since_j2000()),
    ))
}

/// The consecutive solar days that cover `[from, to)`: the first is the day containing
/// `from`, and each later day begins at exactly the previous day's `next_sunrise`, so
/// the days tile the range without a gap or an overlap at the seams.
pub fn solar_days_covering(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    from: Instant,
    to: Instant,
) -> Result<Vec<SolarDay>, DayError> {
    let mut days = Vec::new();
    let mut day = solar_day(spk, eop, obs, from)?.ok_or(DayError::NoSolarDay)?;
    loop {
        days.push(day);
        if day.next_sunrise >= to {
            break;
        }
        let next = solar_day(spk, eop, obs, day.next_sunrise.add_ns(NS_PER_SEC))?
            .ok_or(DayError::NoSolarDay)?;
        day = SolarDay {
            sunrise: day.next_sunrise,
            sunset: next.sunset,
            next_sunrise: next.next_sunrise,
        };
    }
    Ok(days)
}

#[cfg(test)]
pub(crate) mod fixture {
    use kshana_ephem::Spk;
    use kshana_sky::Observer;
    use kshana_time::{Instant, Utc};

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixture::{kernel, utc, KOCHI};
    use kshana_time::NS_PER_DAY;

    #[test]
    fn weekday_matches_the_events_crate_vara() {
        let Some(spk) = kernel() else { return };
        // 2026-03-20 is a Friday
        let day = solar_day(&spk, None, &KOCHI, utc(2026, 3, 20, 6, 0, 0))
            .unwrap()
            .unwrap();
        assert_eq!(weekday(&day, &KOCHI), 5);
        assert_eq!(kshana_events::vara_name(&day, &KOCHI), "Shukravara");
    }

    #[test]
    fn covering_days_tile_without_seams() {
        let Some(spk) = kernel() else { return };
        let from = utc(2026, 10, 15, 9, 0, 0);
        let days =
            solar_days_covering(&spk, None, &KOCHI, from, from.add_ns(3 * NS_PER_DAY)).unwrap();
        assert_eq!(days.len(), 4, "{days:?}");
        for w in days.windows(2) {
            assert_eq!(w[0].next_sunrise, w[1].sunrise);
            assert!(w[1].sunrise < w[1].sunset && w[1].sunset < w[1].next_sunrise);
        }
    }

    #[test]
    fn ascendant_agrees_with_the_chart() {
        let Some(spk) = kernel() else { return };
        let t = utc(2026, 10, 15, 9, 2, 7);
        let ch = kshana_chart::compute(
            &spk,
            None,
            &kshana_chart::ChartInput {
                instant: t,
                observer: KOCHI,
                system: kshana_houses::HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap();
        let asc = ascendant_sidereal(None, &KOCHI, t, Ayanamsa::Lahiri);
        assert!(
            (asc - ch.cusps_sidereal[0]).abs() < 1e-9,
            "{asc} vs {}",
            ch.cusps_sidereal[0]
        );
    }
}
